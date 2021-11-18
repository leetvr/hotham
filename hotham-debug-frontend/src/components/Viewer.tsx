import {
  ArcballControls,
  Box,
  Environment,
  OrbitControls,
  PerspectiveCamera,
} from '@react-three/drei';
import { Canvas } from '@react-three/fiber';
import React, { Suspense, useRef, useState } from 'react';
import styled from 'styled-components';
import { ViewOptions } from './ViewOptions';
import { useGLTF } from '@react-three/drei';
import { GLTF, GLTF as GLTFThree } from 'three/examples/jsm/loaders/GLTFLoader';
import THREE, { Euler, Group, Material, Mesh, Quaternion } from 'three';
import { Entity, Transform } from '../App';

const CanvasContainer = styled.div`
  display: 'flex';
  flex: 4;
  overflow: 'hidden';
  width: '70vw';
`;

type GLTFResult = GLTF & {
  nodes: Record<string, Mesh>;
  materials: Record<string, THREE.MeshPhysicalMaterial>;
};

export interface DisplayOptions {
  models?: boolean;
  physics?: boolean;
}

function Model({
  mesh,
  material,
  transform,
}: {
  mesh: Mesh;
  material: Material;
  transform: Transform;
}): JSX.Element {
  const group = useRef<THREE.Group>();
  const rotation = getRotation(transform);
  return (
    <group ref={group} dispose={null}>
      <mesh
        castShadow
        receiveShadow
        geometry={mesh.geometry}
        material={material}
        position={transform.translation}
        scale={transform.scale}
        rotation={rotation}
        userData={{ name: 'Environment' }}
      />
    </group>
  );
}

function getRotation(t: Transform): Euler {
  const rotation = new Euler();
  rotation.setFromQuaternion(
    new Quaternion(t.rotation[0], t.rotation[1], t.rotation[2], t.rotation[3])
  );

  return rotation;
}

interface Props {
  entities: Record<number, Entity>;
}

function getModels(
  entities: Record<number, Entity>,
  nodes: Record<string, Mesh>,
  materials: Record<string, THREE.MeshPhysicalMaterial>
): JSX.Element[] {
  return Object.values(entities)
    .filter((e) => e.mesh && e.material && e.transform)
    .map((e) => (
      <Model
        key={e.id}
        mesh={nodes[e.mesh!]}
        material={materials[e.material!]}
        transform={e.transform!}
      />
    ));
}

export function Viewer({ entities }: Props): JSX.Element {
  const [displays, setDisplays] = useState<DisplayOptions>({ models: true });
  const gltf = useGLTF('/beat_saber.glb') as unknown as GLTFResult;
  console.log(gltf);
  const { nodes, materials } = gltf;
  return (
    <>
      <ViewOptions displays={displays} setDisplays={setDisplays} />
      <CanvasContainer>
        <Canvas shadows={true}>
          {displays.models && getModels(entities, nodes, materials)}
          {displays.physics && getPhsicsObjects(entities)}
          <Environment preset="studio" />
          <ArcballControls />
        </Canvas>
      </CanvasContainer>
    </>
  );
}
function getPhsicsObjects(
  entities: Record<number, Entity>
): JSX.Element[] | [] {
  const elements: JSX.Element[] = [];
  Object.values(entities).forEach((e) => {
    if (e.collider?.type === 'cube') {
      elements.push(
        <Box
          args={[
            e.collider.geometry[0],
            e.collider.geometry[1],
            e.collider.geometry[2],
          ]}
        >
          <meshPhongMaterial attach="material" color="#bbb" wireframe />
        </Box>
      );
    }
  });
  return elements;
}
