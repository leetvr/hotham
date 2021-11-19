import React, { useEffect, useState } from 'react';
import styled from 'styled-components';
import { LeftPanel } from './components/LeftPanel';
import { RightPanel } from './components/RightPanel';
const SERVER_IP = 'localhost';
const ws = new WebSocket(`ws://${SERVER_IP}:8080`);

enum Command {
  Reset,
  Init,
}

interface InitData {
  data: Frame;
  session_id: number;
}

interface Message {
  Data: Frame;
  Command: Command;
  Init: InitData;
  Error: string;
}

type Entities = Record<number, Entity>;
interface Frame {
  id: number;
  entities: Entities;
}

const Container = styled.div`
  display: flex;
  flex: 1;
  flex-direction: row;
  height: 100vh;
  width: 100vw;
`;

function App() {
  const [frames, setFrames] = useState<Frame[]>([]);
  const [error, setError] = useState<string | undefined>();
  useEffect(() => {
    ws.onopen = () => {
      ws.send(JSON.stringify({ Command: Command.Init }));
    };
  });
  useEffect(() => {
    ws.onmessage = (m) => {
      const message: Message = JSON.parse(m.data);
      if (message.Data) {
        if (message.Data) {
          console.log('Received  data!');
          setFrames((f) => [...f, message.Data]);
        }
      }
      if (message.Init) {
        setFrames((f) => [...f, message.Init.data]);
      }
      if (message.Error) {
        setError(error);
      }
    };
  });

  const [currentFrame, setCurrentFrame] = useState(0);

  // const frames: Frame[] = [
  //   {
  //     id: 0,
  //     entities: {
  //       0: {
  //         name: 'Environment',
  //         id: 0,
  //         mesh: 'Environment',
  //         material: 'Rough',
  //         transform: {
  //           translation: [0, 0, -1],
  //           rotation: [0, 0, 0],
  //           scale: [1, 1, 1],
  //         },
  //         collider: {
  //           colliderType: 'cube',
  //           geometry: [1, 1, 1],
  //         },
  //       },
  //       1: { name: 'Empty', id: 1 },
  //     },
  //   },
  //   {
  //     id: 1,
  //     entities: {
  //       0: {
  //         name: 'Environment',
  //         id: 0,
  //         mesh: 'Environment',
  //         material: 'Rough',
  //         transform: {
  //           translation: [0, 0, -1.1],
  //           rotation: [0, 0, 0],
  //           scale: [1, 1, 1],
  //         },
  //         collider: {
  //           colliderType: 'cube',
  //           geometry: [1, 1, 1],
  //         },
  //       },
  //     },
  //   },
  // ];

  const entities = frames[currentFrame] ? frames[currentFrame].entities : [];

  return (
    <Container>
      <LeftPanel
        entities={entities}
        frame={currentFrame}
        setFrame={setCurrentFrame}
        maxFrames={frames.length}
      />
      <RightPanel entities={entities} />
    </Container>
  );
}

export interface Transform {
  translation: [number, number, number];
  rotation: [number, number, number];
  scale: [number, number, number];
}

export interface Entity {
  id: number;
  name: string;
  transform?: Transform;
  collider?: {
    colliderType: 'cube' | 'cylinder';
    geometry: number[];
  };
}

export default App;
