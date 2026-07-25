// Rationale — language spike (Go candidate).
//
// Implementa exactamente las 6 operaciones de
// docs/research/language/spike-protocol.md, ni más ni menos:
//  1. Leer un Record (YAML) desde disco.
//  2. Abrir SQLite (crear, insertar, leer una fila).
//  3. Llamar/mockear un proveedor estructural externo (subprocess, con deadline).
//  4. Verificar una revisión (comparar dos strings).
//  5. Rankear una constraint (ordenar una lista pequeña por campo numérico).
//  6. Emitir JSON a stdout.
//
// Además prueba, fuera del pipeline principal: servidor MCP mínimo (--serve),
// file locking, subprocess con cancelación real por deadline (--demo-timeout).
package main

import (
	"bufio"
	"bytes"
	"database/sql"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"syscall"
	"time"

	_ "modernc.org/sqlite"
	"gopkg.in/yaml.v3"
)

type Record struct {
	ID             string `yaml:"id" json:"id"`
	Kind           string `yaml:"kind" json:"kind"`
	Severity       string `yaml:"severity" json:"severity"`
	Statement      string `yaml:"statement" json:"statement"`
	Subject        string `yaml:"subject" json:"subject"`
	BoundRevision  string `yaml:"bound_revision" json:"bound_revision"`
}

type RankedConstraint struct {
	ID             string `json:"id"`
	SeverityWeight int64  `json:"severity_weight"`
	Statement      string `json:"statement"`
}

type OutputPacket struct {
	RecordID            string                 `json:"record_id"`
	RevisionConsistency string                 `json:"revision_consistency"`
	Provider            map[string]interface{} `json:"provider"`
	TopConstraint       RankedConstraint       `json:"top_constraint"`
	LatencyMs           int64                  `json:"latency_ms"`
}

func severityWeight(s string) int64 {
	switch s {
	case "critical":
		return 100
	case "high":
		return 75
	case "medium":
		return 50
	case "low":
		return 25
	default:
		return 0
	}
}

// --- Operación 1: leer un Record YAML desde disco ---
func op1ReadRecord(path string) Record {
	data, err := os.ReadFile(path)
	if err != nil {
		panic(fmt.Sprintf("no se pudo leer el Record YAML: %v", err))
	}
	var r Record
	if err := yaml.Unmarshal(data, &r); err != nil {
		panic(fmt.Sprintf("Record YAML inválido: %v", err))
	}
	return r
}

// --- Operación 2: SQLite — crear, insertar, leer una fila ---
func op2SqliteRoundtrip(dbPath string, record Record) string {
	db, err := sql.Open("sqlite", dbPath)
	if err != nil {
		panic(fmt.Sprintf("no se pudo abrir sqlite: %v", err))
	}
	defer db.Close()

	if _, err := db.Exec(`CREATE TABLE IF NOT EXISTS records (id TEXT PRIMARY KEY, statement TEXT NOT NULL)`); err != nil {
		panic(fmt.Sprintf("create table: %v", err))
	}
	if _, err := db.Exec(`INSERT OR REPLACE INTO records (id, statement) VALUES (?, ?)`, record.ID, record.Statement); err != nil {
		panic(fmt.Sprintf("insert: %v", err))
	}
	var statement string
	if err := db.QueryRow(`SELECT statement FROM records WHERE id = ?`, record.ID).Scan(&statement); err != nil {
		panic(fmt.Sprintf("select: %v", err))
	}
	return statement
}

// --- Operación 3: llamar/mockear el proveedor estructural (subprocess) ---
//
// Hallazgo real durante el spike (ver docs/research/language/spike-notes.md):
// la forma idiomática ingenua — exec.CommandContext + cmd.Output() — NO
// cancela a tiempo cuando el script hijo (bash) spawnea un nieto (`sleep`)
// que hereda el file descriptor de escritura del pipe de stdout. Matar solo
// al hijo directo (bash) no cierra ese descriptor: el nieto sigue vivo y
// cmd.Output() sigue bloqueado leyendo el pipe hasta EOF, que no llega hasta
// que el nieto termina por sí solo (el sleep completo de 5s). Es el clásico
// problema Unix del "huérfano que sostiene el pipe abierto".
//
// La corrección correcta y portable en POSIX es matar el *grupo de procesos*
// completo (Setpgid + kill(-pid)), no solo el proceso hijo directo. Con esa
// corrección, la cancelación sí ocurre en ~500ms como se espera.
func op3CallProvider(script string, slow bool, deadline time.Duration) map[string]interface{} {
	args := []string{}
	if slow {
		args = append(args, "slow")
	}
	cmd := exec.Command(script, args...)
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	var outBuf bytes.Buffer
	cmd.Stdout = &outBuf // buffer, no pipe: Wait() lo llena directamente, sin
	// la carrera "Wait() cierra el pipe antes de que el lector externo termine".

	if err := cmd.Start(); err != nil {
		return map[string]interface{}{"status": "malformed"}
	}

	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()

	select {
	case <-time.After(deadline):
		// Matar todo el grupo de procesos, no solo el hijo directo, para
		// que ningún nieto huérfano mantenga el pipe abierto.
		_ = syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL)
		<-done // reap
		return map[string]interface{}{
			"provider": "mock",
			"status":   "timeout",
			"coverage": "unknown",
		}
	case err := <-done:
		if err != nil {
			return map[string]interface{}{"status": "malformed"}
		}
		var result map[string]interface{}
		if err := json.Unmarshal(outBuf.Bytes(), &result); err != nil {
			return map[string]interface{}{"status": "malformed"}
		}
		return result
	}
}

// --- Operación 4: verificar una revisión (comparar dos strings) ---
func op4CheckRevision(recordRevision, providerRevision string) string {
	if recordRevision == providerRevision {
		return "exact"
	}
	return "structural-index-behind"
}

// --- Operación 5: rankear una constraint (ordenar por campo numérico) ---
func op5RankConstraints(record Record) RankedConstraint {
	candidates := []RankedConstraint{
		{
			ID:             "risk.staff-without-assignment",
			SeverityWeight: severityWeight("medium"),
			Statement:      "Staff without entity assignments may lose access.",
		},
		{
			ID:             record.ID,
			SeverityWeight: severityWeight(record.Severity),
			Statement:      record.Statement,
		},
		{
			ID:             "decision.staff-access-is-entity-scoped",
			SeverityWeight: severityWeight("high"),
			Statement:      "Staff access must be assigned independently per entity.",
		},
	}
	sort.Slice(candidates, func(i, j int) bool {
		return candidates[i].SeverityWeight > candidates[j].SeverityWeight
	})
	return candidates[0]
}

// --- File locking: lock advisorio sobre un archivo de estado (POSIX) ---
// Demuestra la capacidad, no forma parte del pipeline de las 6 operaciones.
// Nota de comparabilidad: syscall.Flock es POSIX-only (macOS/Linux); en
// Windows requeriría una implementación separada (LockFileEx) — a diferencia
// del std::fs file locking usado en el candidato Rust, que sí es portable
// entre los tres sistemas operativos objetivo desde la librería estándar.
func demoFileLock(path string) error {
	if runtime.GOOS == "windows" {
		return fmt.Errorf("flock POSIX no soportado en windows en este spike")
	}
	f, err := os.Create(path)
	if err != nil {
		return err
	}
	return syscall.Flock(int(f.Fd()), syscall.LOCK_EX|syscall.LOCK_NB)
}

func runPipeline(fixturesDir, workDir string, slowProvider bool) OutputPacket {
	t0 := time.Now()

	record := op1ReadRecord(filepath.Join(fixturesDir, "record.yaml"))
	_ = op2SqliteRoundtrip(filepath.Join(workDir, "spike.sqlite3"), record)

	provider := op3CallProvider(filepath.Join(fixturesDir, "mock_provider.sh"), slowProvider, 500*time.Millisecond)
	providerRevision, _ := provider["indexed_revision"].(string)
	if providerRevision == "" {
		providerRevision = "unknown"
	}
	consistency := op4CheckRevision(record.BoundRevision, providerRevision)

	top := op5RankConstraints(record)

	return OutputPacket{
		RecordID:            record.ID,
		RevisionConsistency: consistency,
		Provider:            provider,
		TopConstraint:       top,
		LatencyMs:           time.Since(t0).Milliseconds(),
	}
}

// --- MCP mínimo: JSON-RPC 2.0 framed con Content-Length sobre stdio ---
// Mismo framing verificado empíricamente contra Codebase Memory
// (docs/research/codebase-memory/11-performance-observations.md).
func readMCPMessage(r *bufio.Reader) (map[string]interface{}, error) {
	var length int
	for {
		line, err := r.ReadString('\n')
		if err != nil {
			return nil, err
		}
		line = strings.TrimRight(line, "\r\n")
		if line == "" {
			break
		}
		if strings.HasPrefix(strings.ToLower(line), "content-length:") {
			parts := strings.SplitN(line, ":", 2)
			length, _ = strconv.Atoi(strings.TrimSpace(parts[1]))
		}
	}
	body := make([]byte, length)
	if _, err := io.ReadFull(r, body); err != nil {
		return nil, err
	}
	var msg map[string]interface{}
	if err := json.Unmarshal(body, &msg); err != nil {
		return nil, err
	}
	return msg, nil
}

func writeMCPMessage(w io.Writer, value interface{}) {
	body, _ := json.Marshal(value)
	fmt.Fprintf(w, "Content-Length: %d\r\n\r\n%s", len(body), body)
}

func serveMCP(fixturesDir, workDir string) {
	reader := bufio.NewReader(os.Stdin)
	writer := os.Stdout

	for {
		msg, err := readMCPMessage(reader)
		if err != nil {
			return
		}
		method, _ := msg["method"].(string)
		id := msg["id"]

		switch method {
		case "initialize":
			writeMCPMessage(writer, map[string]interface{}{
				"jsonrpc": "2.0",
				"id":      id,
				"result": map[string]interface{}{
					"protocolVersion": "2024-11-05",
					"capabilities":    map[string]interface{}{"tools": map[string]interface{}{}},
					"serverInfo":      map[string]interface{}{"name": "rationale-spike-go", "version": "0.0.0"},
				},
			})
		case "tools/call":
			packet := runPipeline(fixturesDir, workDir, false)
			packetJSON, _ := json.Marshal(packet)
			writeMCPMessage(writer, map[string]interface{}{
				"jsonrpc": "2.0",
				"id":      id,
				"result": map[string]interface{}{
					"content": []map[string]interface{}{
						{"type": "text", "text": string(packetJSON)},
					},
					"isError": false,
				},
			})
		case "notifications/initialized":
			// sin respuesta, es una notificación
		default:
			if id != nil {
				writeMCPMessage(writer, map[string]interface{}{
					"jsonrpc": "2.0",
					"id":      id,
					"error":   map[string]interface{}{"code": -32601, "message": "method not found"},
				})
			}
		}
	}
}

func main() {
	exe, err := os.Executable()
	if err != nil {
		panic(err)
	}
	base := filepath.Dir(exe)
	// go run compila a un binario temporal; para el spike, resolver
	// fixtures relativas al cwd si el binario no vive junto al proyecto.
	if _, err := os.Stat(filepath.Join(base, "fixtures")); err != nil {
		cwd, _ := os.Getwd()
		base = cwd
	}
	fixturesDir := filepath.Join(base, "fixtures")
	workDir := filepath.Join(base, "spike-work")
	os.MkdirAll(workDir, 0o755)

	args := os.Args[1:]
	for _, a := range args {
		switch a {
		case "--serve":
			serveMCP(fixturesDir, workDir)
			return
		case "--demo-timeout":
			t0 := time.Now()
			provider := op3CallProvider(filepath.Join(fixturesDir, "mock_provider.sh"), true, 500*time.Millisecond)
			out, _ := json.Marshal(map[string]interface{}{
				"elapsed_ms": time.Since(t0).Milliseconds(),
				"provider":   provider,
			})
			fmt.Println(string(out))
			return
		case "--demo-lock":
			lockPath := filepath.Join(workDir, "spike.lock")
			if err := demoFileLock(lockPath); err != nil {
				out, _ := json.Marshal(map[string]interface{}{"lock": "failed", "error": err.Error()})
				fmt.Println(string(out))
			} else {
				out, _ := json.Marshal(map[string]interface{}{"lock": "acquired"})
				fmt.Println(string(out))
			}
			return
		}
	}

	packet := runPipeline(fixturesDir, workDir, false)
	out, _ := json.Marshal(packet)
	fmt.Println(string(out))
}
