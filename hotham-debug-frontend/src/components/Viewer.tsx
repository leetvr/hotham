import { Box, Environment } from '@react-three/drei';
import { Canvas } from '@react-three/fiber';
import React, { Suspense, useRef, useState } from 'react';
import styled from 'styled-components';
import { ViewOptions } from './ViewOptions';
import { useGLTF } from '@react-three/drei';
import { GLTF, GLTF as GLTFThree } from 'three/examples/jsm/loaders/GLTFLoader';
import { Group, Material, Mesh } from 'three';

const CanvasContainer = styled.div`
  display: 'flex';
  flex: 5;
  overflow: 'hidden';
  width: '70vw';
`;

type GLTFResult = GLTF & {
  nodes: {
    Environment: THREE.Mesh;
    Cylinder: THREE.Mesh;
    Cylinder_1: THREE.Mesh;
    Cylinder001: THREE.Mesh;
    Cylinder001_1: THREE.Mesh;
    Cube001: THREE.Mesh;
    Cube001_1: THREE.Mesh;
    Cube003: THREE.Mesh;
    Cube003_1: THREE.Mesh;
    Cylinder004: THREE.Mesh;
    Cylinder004_1: THREE.Mesh;
  };
  materials: {
    Rough: THREE.MeshStandardMaterial;
    Glow: THREE.MeshStandardMaterial;
  };
};

interface IModel {
  mesh: THREE.Mesh;
  material: THREE.MeshStandardMaterial;
}

export interface DisplayOptions {
  models?: boolean;
  physics?: boolean;
}

function Model({ mesh, material }: IModel): JSX.Element {
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

export function Viewer(): JSX.Element {
  const [displays, setDisplays] = useState<DisplayOptions>({});
  const gltf = useGLTF('/beat_saber.glb') as unknown as GLTFResult;
  console.log(gltf);
  const { nodes, materials } = gltf;
  return (
    <>
      <ViewOptions displays={displays} setDisplays={setDisplays} />
      <CanvasContainer>
        <Canvas shadows={true}>
          <Model mesh={nodes.Environment} material={materials.Rough} />
          <Environment preset="studio" />
        </Canvas>
      </CanvasContainer>
    </>
  );
}
