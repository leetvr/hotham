import { useLiveQuery } from 'dexie-react-hooks';
import React, { useEffect, useState } from 'react';
import styled from 'styled-components';
import { EntityList } from './components/EntityList';
import { Inspector } from './components/Inspector';
import { SessionSelector } from './components/SessionSelector';
import { Timeline } from './components/Timeline';
import { db } from './db';
export const SERVER_ADDRESS = `ws://localhost:8000`;

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
  id: number;
  entities: Entities;
  sessionId: number;
}

export interface Session {
  id: number;
  timestamp: Date;
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
  // State
  const [selectedSessionId, setSelectedSessionId] = useState<
    number | undefined
  >();
  const [selectedFrameId, setSelectedFrameId] = useState(0);
  const [selectedEntity, setSelectedEntity] = useState<Entity | undefined>();
  const [connected, setConnected] = useState(false);

  // Websocket
  useEffect(() => {
    const ws = new WebSocket(SERVER_ADDRESS);
    ws.onopen = () => {
      setConnected(true);
      ws.send(JSON.stringify({ Command: Command.Init }));
    };
    ws.onclose = () => {
      setConnected(false);
    };
  });

  // Database
  const sessions = useLiveQuery(() => db.sessions.toArray()) ?? [];
  const frames =
    useLiveQuery(
      () =>
        db.frames
          .where('sessionId')
          .equals(selectedSessionId ?? -1)
          .toArray(),
      [selectedSessionId]
    ) ?? [];

  const entities = frames[selectedFrameId]?.entities ?? [];

  return (
    <Container>
      <SessionSelector
        sessions={sessions}
        setSelectedSessionId={setSelectedSessionId}
        connected={connected}
      />
      <EntityList entities={entities} setSelectedEntity={setSelectedEntity} />
      <Inspector entity={selectedEntity} />
      <Timeline
        maxFrames={frames.length}
        setSelectedFrameId={setSelectedFrameId}
        selectedFrameId={selectedFrameId}
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

// useEffect(() => {
//   ws.onmessage = (m) => {
//     const message: Message = JSON.parse(m.data);
//     if (message.Data) {
//       if (message.Data) {
//         setFrames((f) => {
//           const updated = [...f, message.Data];
//           localStorage.setItem(sessionId.toString(), JSON.stringify(updated));
//           return updated;
//         });
//       }
//     }
//     if (message.Init) {
//       setFrames((f) => [...f, message.Init.data]);
//       const { session_id } = message.Init;
//       setSessionId(session_id);
//       const sessionsRaw = localStorage.getItem('sessions');
//       const sessions = sessionsRaw ? JSON.parse(sessionsRaw) : [];
//       const updated = [...sessions, session_id];
//       localStorage.setItem('sessions', JSON.stringify(updated));
//     }
//     if (message.Error) {
//       setError(error);
//     }
//   };
// });
