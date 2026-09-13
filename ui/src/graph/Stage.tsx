/*
 * Adapted from codebase-memory-mcp graph-ui — src/components/GraphScene.tsx
 * (eased camera fly-to that also moves the orbit pivot, idle auto-rotation,
 * bloom over a dark scene). Copyright (c) 2025 DeusData. MIT License — see
 * THIRD_PARTY.md. Rationale uses three's own OrbitControls and
 * UnrealBloomPass instead of drei/postprocessing to keep the dependency
 * surface small, and adds eased motion toward the browser-side layout.
 */
import { useFrame, useThree } from "@react-three/fiber";
import { useEffect, useMemo, useRef, type RefObject } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { EffectComposer } from "three/examples/jsm/postprocessing/EffectComposer.js";
import { OutputPass } from "three/examples/jsm/postprocessing/OutputPass.js";
import { RenderPass } from "three/examples/jsm/postprocessing/RenderPass.js";
import { UnrealBloomPass } from "three/examples/jsm/postprocessing/UnrealBloomPass.js";
import type { Vec3 } from "./layout";

export interface CameraTarget {
  position: THREE.Vector3;
  lookAt: THREE.Vector3;
}

const IDLE_ROTATE_MS = 45_000;

export function CameraRig({ target }: { target: CameraTarget | null }) {
  const camera = useThree((state) => state.camera);
  const domElement = useThree((state) => state.gl.domElement);
  const controls = useMemo(() => new OrbitControls(camera, domElement), [camera, domElement]);
  const flight = useRef<{ target: CameraTarget; progress: number } | null>(null);
  const lastInteraction = useRef(performance.now());

  useEffect(() => {
    controls.enableDamping = true;
    controls.dampingFactor = 0.08;
    controls.rotateSpeed = 0.5;
    controls.zoomSpeed = 1.1;
    controls.minDistance = 20;
    controls.maxDistance = 6000;
    controls.autoRotateSpeed = 0.3;
    const interrupt = () => {
      lastInteraction.current = performance.now();
      flight.current = null;
    };
    controls.addEventListener("start", interrupt);
    return () => {
      controls.removeEventListener("start", interrupt);
      controls.dispose();
    };
  }, [controls]);

  useEffect(() => {
    if (target) {
      flight.current = { target, progress: 0 };
      lastInteraction.current = performance.now();
    }
  }, [target]);

  useFrame(() => {
    const current = flight.current;
    if (current && current.progress < 1) {
      current.progress = Math.min(1, current.progress + 0.022);
      const eased = 1 - Math.pow(1 - current.progress, 3);
      camera.position.lerp(current.target.position, eased * 0.09);
      // El pivote también viaja: si no, OrbitControls vuelve a centrar la
      // vista en el origen en el siguiente frame.
      controls.target.lerp(current.target.lookAt, eased * 0.09);
    }
    controls.autoRotate = performance.now() - lastInteraction.current > IDLE_ROTATE_MS;
    controls.update();
  });

  return null;
}

export function Bloom({ strength = 0.95 }: { strength?: number }) {
  const gl = useThree((state) => state.gl);
  const scene = useThree((state) => state.scene);
  const camera = useThree((state) => state.camera);
  const size = useThree((state) => state.size);

  const { composer, bloom } = useMemo(() => {
    const composer = new EffectComposer(gl);
    composer.addPass(new RenderPass(scene, camera));
    const bloom = new UnrealBloomPass(new THREE.Vector2(256, 256), 0.95, 0.55, 0.12);
    composer.addPass(bloom);
    composer.addPass(new OutputPass());
    return { composer, bloom };
  }, [gl, scene, camera]);

  useEffect(() => {
    bloom.strength = strength;
  }, [bloom, strength]);

  useEffect(() => {
    composer.setPixelRatio(gl.getPixelRatio());
    composer.setSize(Math.max(1, size.width), Math.max(1, size.height));
  }, [composer, gl, size]);

  useEffect(() => () => composer.dispose(), [composer]);

  // Prioridad 1: el composer toma el render y R3F deja de pintar por su cuenta.
  useFrame(() => composer.render(), 1);
  return null;
}

/** Posiciones vivas que se acercan con suavidad a las del layout. */
export function Motion({
  targets,
  live,
}: {
  targets: Map<string, Vec3>;
  live: RefObject<Map<string, Vec3>>;
}) {
  useFrame((_, delta) => {
    const factor = 1 - Math.pow(0.002, Math.min(delta, 0.1));
    const current = live.current;
    for (const [key, target] of targets) {
      const position = current.get(key);
      if (!position) {
        current.set(key, { x: target.x, y: target.y, z: target.z });
        continue;
      }
      position.x += (target.x - position.x) * factor;
      position.y += (target.y - position.y) * factor;
      position.z += (target.z - position.z) * factor;
    }
    for (const key of [...current.keys()]) {
      if (!targets.has(key)) current.delete(key);
    }
  });
  return null;
}

/** Las aristas son líneas de 1px: sin tolerancia, nunca se podrían clicar. */
export function RaycastTuning() {
  const raycaster = useThree((state) => state.raycaster);
  useEffect(() => {
    const previous = raycaster.params.Line?.threshold ?? 1;
    raycaster.params.Line = { threshold: 3 };
    return () => {
      raycaster.params.Line = { threshold: previous };
    };
  }, [raycaster]);
  return null;
}

/** Polvo de fondo: profundidad sin competir con el grafo. */
export function Starfield({ count = 900, radius = 2800 }: { count?: number; radius?: number }) {
  const geometry = useMemo(() => {
    let seed = 7;
    const random = () => {
      seed = (seed * 16807) % 2147483647;
      return seed / 2147483647;
    };
    const positions = new Float32Array(count * 3);
    for (let index = 0; index < count; index += 1) {
      const r = radius * (0.3 + 0.7 * random());
      const theta = random() * Math.PI * 2;
      const phi = Math.acos(2 * random() - 1);
      positions[index * 3] = r * Math.sin(phi) * Math.cos(theta);
      positions[index * 3 + 1] = r * Math.sin(phi) * Math.sin(theta);
      positions[index * 3 + 2] = r * Math.cos(phi);
    }
    const buffer = new THREE.BufferGeometry();
    buffer.setAttribute("position", new THREE.BufferAttribute(positions, 3));
    return buffer;
  }, [count, radius]);
  useEffect(() => () => geometry.dispose(), [geometry]);

  return (
    <points geometry={geometry} raycast={() => null} frustumCulled={false}>
      <pointsMaterial
        size={1.4}
        color="#31405a"
        sizeAttenuation
        transparent
        opacity={0.6}
        depthWrite={false}
        fog={false}
      />
    </points>
  );
}
