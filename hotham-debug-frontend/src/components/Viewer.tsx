import { Box, Cylinder, Environment, OrbitControls } from '@react-three/drei';
import { Canvas } from '@react-three/fiber';
import { useRef, useState } from 'react';
import styled from 'styled-components';
import { ViewOptions } from './ViewOptions';
import { useGLTF } from '@react-three/drei';
import { GLTF } from 'three/examples/jsm/loaders/GLTFLoader';
import THREE, { Matrix4, Mesh, Vector3 } from 'three';
import { Entity, Transform } from '../App';
import { vec4toQuaternion } from '../util';

const CanvasContainer = styled.div`
  display: flex;
  overflow: hidden;
  width: 80vw;
  height: 80vh;
  flex: 1;
`;

const OuterContainer = styled.div`
  display: 'flex';
  flex-direction: 'column';
`;

type GLTFResult = GLTF & {
  nodes: Record<string, Mesh>;
};

export interface DisplayOptions {
  models?: boolean;
  physics?: boolean;
}

function Model({
  mesh,
  transform,
}: {
  mesh: Mesh;
  transform: Transform;
}): JSX.Element {
  const group = useRef<THREE.Group>();
  const { translation: t, rotation: r, scale: s } = transform;
  const translation = new Vector3(t[0], t[1], t[2]);
  const rotation = vec4toQuaternion(r);
  const scale = new Vector3(s[0], s[1], s[2]);
  const matrix = new Matrix4().compose(translation, rotation, scale);

  return (
    <group ref={group} dispose={null} matrix={matrix} matrixAutoUpdate={false}>
      <mesh
        castShadow
        receiveShadow
        geometry={mesh.geometry}
        material={mesh.material}
        userData={{ name: 'Environment' }}
      />
    </group>
  );
}

interface Props {
  entities: Entity[];
}

function getModels(
  entities: Entity[],
  nodes: Record<string, Mesh>
): JSX.Element[] | [] {
  const elements: JSX.Element[] = [];
  for (let e of entities) {
    const key = e.name.replaceAll(' ', '_');
    const node = nodes[key];
    if (!node) {
      console.warn('No node found for', key);
      continue;
    }

    if (node.children.length) {
      for (let child of node.children) {
        const m = child as Mesh;
        elements.push(
          <Model
            key={`${e.id}_${child.id}`}
            mesh={m}
            transform={e.transform!}
          />
        );
      }
    } else {
      elements.push(
        <Model
          key={`${e.id}_${node.id}`}
          mesh={node}
          transform={e.transform!}
        />
      );
    }
  }

  return elements;
}

export function Viewer({ entities }: Props): JSX.Element {
  const [displays, setDisplays] = useState<DisplayOptions>({ models: true });
  const gltf = useGLTF('/beat_saber.glb') as unknown as GLTFResult;
  const { nodes } = gltf;
  return (
    <OuterContainer>
      <ViewOptions displays={displays} setDisplays={setDisplays} />
      <CanvasContainer>
        <Canvas shadows={true}>
          {displays.models && getModels(entities, nodes)}
          {displays.physics && getPhsicsObjects(entities)}
          <Environment preset="studio" />
          <OrbitControls />
        </Canvas>
      </CanvasContainer>
    </OuterContainer>
  );
}
function getPhsicsObjects(
  entities: Record<number, Entity>
): JSX.Element[] | [] {
  const elements: JSX.Element[] = [];
  Object.values(entities).forEach((e) => {
    const { collider } = e;
    if (!collider) return;

    const { colliderType, geometry, translation: t, rotation: r } = collider;
    const rotationQuaternion = vec4toQuaternion(r);
    const transformMatrix = new Matrix4()
      .makeRotationFromQuaternion(rotationQuaternion)
      .setPosition(t[0], t[1], t[2]);

    if (colliderType === 'cube') {
      elements.push(
        <Box
          matrixAutoUpdate={false}
          matrix={transformMatrix}
          args={[geometry[0] * 2, geometry[1] * 2, geometry[2] * 2]}
        >
          <meshPhongMaterial attach="material" color="#bbb" wireframe />
        </Box>
      );
    }

    if (colliderType === 'cylinder') {
      const height = geometry[0] * 2;
      const radius = geometry[1];
      elements.push(
        <Cylinder
          args={[radius, radius, height]}
          matrixAutoUpdate={false}
          matrix={transformMatrix}
        >
          <meshPhongMaterial attach="material" color="#bbb" wireframe />
        </Cylinder>
      );
    }
  });
  return elements;
}
