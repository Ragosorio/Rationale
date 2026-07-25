package main

import (
	"os"
	"path/filepath"
	"testing"
)

func TestOp4RevisionExact(t *testing.T) {
	if got := op4CheckRevision("abc123", "abc123"); got != "exact" {
		t.Fatalf("expected exact, got %s", got)
	}
}

func TestOp4RevisionBehind(t *testing.T) {
	if got := op4CheckRevision("abc123", "def456"); got != "structural-index-behind" {
		t.Fatalf("expected structural-index-behind, got %s", got)
	}
}

func TestOp5RanksCriticalHighest(t *testing.T) {
	record := Record{ID: "constraint.test", Severity: "critical", Statement: "test statement"}
	top := op5RankConstraints(record)
	if top.ID != "constraint.test" || top.SeverityWeight != 100 {
		t.Fatalf("unexpected top constraint: %+v", top)
	}
}

func TestSeverityWeightMonotonic(t *testing.T) {
	ordered := []string{"low", "medium", "high", "critical"}
	var prev int64 = -1
	for _, s := range ordered {
		w := severityWeight(s)
		if w <= prev {
			t.Fatalf("severity_weight debe ser estrictamente creciente: %s = %d, prev = %d", s, w, prev)
		}
		prev = w
	}
}

func TestOp1ReadsFixtureRecord(t *testing.T) {
	cwd, _ := os.Getwd()
	record := op1ReadRecord(filepath.Join(cwd, "fixtures", "record.yaml"))
	if record.ID != "constraint.no-global-admin-for-staff" {
		t.Fatalf("unexpected record id: %s", record.ID)
	}
	if record.Severity != "critical" {
		t.Fatalf("unexpected severity: %s", record.Severity)
	}
}

func TestOp2SqliteRoundtrip(t *testing.T) {
	dir := t.TempDir()
	record := Record{ID: "constraint.sqlite-test", Severity: "high", Statement: "roundtrip statement"}
	statement := op2SqliteRoundtrip(filepath.Join(dir, "test.sqlite3"), record)
	if statement != "roundtrip statement" {
		t.Fatalf("unexpected statement: %s", statement)
	}
}

// Fuzz test nativo (Go 1.18+, sin dependencia externa) sobre severityWeight:
// invariante — cualquier string desconocido debe rankear con peso 0 (nunca
// negativo, nunca mayor que "critical").
func FuzzSeverityWeight(f *testing.F) {
	seeds := []string{"critical", "high", "medium", "low", "", "unknown", "CRITICAL"}
	for _, s := range seeds {
		f.Add(s)
	}
	f.Fuzz(func(t *testing.T, s string) {
		w := severityWeight(s)
		if w < 0 || w > 100 {
			t.Fatalf("severity_weight fuera de rango para input %q: %d", s, w)
		}
	})
}
