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
import { Group, Material, Mesh } from 'three';
import { Entity } from '../App';

const CanvasContainer = styled.div`
  display: 'flex';
  flex: 4;
  overflow: 'hidden';
  width: '70vw';
`;

type GLTFResult = GLTF & {
  nodes: Record<string, Mesh>;
  materials: Record<string, THREE.MeshStandardMaterial>;
};

export interface DisplayOptions {
  models?: boolean;
  physics?: boolean;
}

function Model({
  mesh,
  material,
}: {
  mesh: Mesh;
  material: Material;
}): JSX.Element {
  const group = useRef<THREE.Group>();
  return (
    <group ref={group} dispose={null}>
      <mesh
        castShadow
        receiveShadow
        geometry={mesh.geometry}
        material={material}
        position={[0, 0, 0]}
        userData={{ name: 'Environment' }}
      />
    </group>
  );
}

interface Props {
  entities: Record<number, Entity>;
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
          {displays.models && (
            <Model mesh={nodes.Environment} material={materials.Rough} />
          )}
          {displays.physics && (
            <Box args={[1, 1, 1]}>
              <meshBasicMaterial attach="material" color="#f3f3f3" wireframe />
            </Box>
          )}
          <Environment preset="studio" />
          <ArcballControls />
        </Canvas>
      </CanvasContainer>
    </>
  );
}
