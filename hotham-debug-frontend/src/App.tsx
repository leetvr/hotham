import React, { useEffect, useState } from 'react';
import styled from 'styled-components';
import { LeftPanel } from './components/LeftPanel';
import { RightPanel } from './components/RightPanel';
const SERVER_IP = 'localhost';
const ws = new WebSocket(`ws://${SERVER_IP}:8000`);

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

export type Entities = Record<number, Entity>;
export interface Frame {
  frame: number;
  entities: Entities;
}

const Container = styled.div`
  display: flex;
  flex: 1;
  flex-direction: row;
  height: 100vh;
  width: 100vw;
  background-color: #2d3439;
`;

function App() {
  const [connected, setConnected] = useState(false);
  const [sessionId, setSessionId] = useState<number>(0);
  const [frames, setFrames] = useState<Frame[]>([]);
  const [error, setError] = useState<string | undefined>();
  useEffect(() => {
    ws.onopen = () => {
      setConnected(true);
      ws.send(JSON.stringify({ Command: Command.Init }));
    };
    ws.onclose = () => {
      setConnected(false);
    };
  });
  useEffect(() => {
    ws.onmessage = (m) => {
      const message: Message = JSON.parse(m.data);
      if (message.Data) {
        if (message.Data) {
          setFrames((f) => {
            const updated = [...f, message.Data];
            localStorage.setItem(sessionId.toString(), JSON.stringify(updated));
            return updated;
          });
        }
      }
      if (message.Init) {
        setFrames((f) => [...f, message.Init.data]);
        const { session_id } = message.Init;
        setSessionId(session_id);
        const sessionsRaw = localStorage.getItem('sessions');
        const sessions = sessionsRaw ? JSON.parse(sessionsRaw) : [];
        const updated = [...sessions, session_id];
        localStorage.setItem('sessions', JSON.stringify(updated));
      }
      if (message.Error) {
        setError(error);
      }
    };
  });
  useEffect(() => {
    const framesFromStorage = localStorage.getItem(sessionId.toString());
    if (!framesFromStorage) return;
    setFrames(JSON.parse(framesFromStorage));
  }, [connected, sessionId]);

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
        maxFrames={frames.length !== 0 ? frames.length - 1 : 0}
      />
      <RightPanel
        entities={entities}
        connected={connected}
        sessionId={sessionId}
        setSessionId={setSessionId}
      />
    </Container>
  );
}

export interface Transform {
  translation: [number, number, number];
  rotation: [number, number, number];
  scale: [number, number, number];
}

export interface Collider {
  colliderType: 'cube' | 'cylinder';
  geometry: number[];
}

export interface Entity {
  id: number;
  name: string;
  transform?: Transform;
  collider?: Collider;
}

export default App;
