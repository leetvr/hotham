import { useLiveQuery } from 'dexie-react-hooks';
import React, { Suspense, useEffect, useState } from 'react';
import styled from 'styled-components';
import { EntityList } from './components/EntityList';
import { Inspector } from './components/Inspector';
import { SessionSelector } from './components/SessionSelector';
import { Timeline } from './components/Timeline';
import { Viewer } from './components/Viewer';
import { db } from './db';
import { createOnMessage, SERVER_ADDRESS } from './ws';

enum Command {
  Reset,
  Init,
}

export interface InitData {
  firstFrame: Frame;
  sessionId: string;
}

export interface Message {
  frame?: Frame;
  command?: Command;
  init?: InitData;
  error?: string;
}

export type Entities = Record<number, Entity>;
export interface Frame {
  id: string;
  frameNumber: number;
  entities: Entities;
  sessionId: string;
}

export interface Session {
  id: string;
  timestamp: Date;
}

const OuterContainer = styled.div`
  display: flex;
  flex: 1;
  height: 100vh;
  width: 100vw;
  flex-direction: column;
  background-color: #2d3439;
`;

const TopContainer = styled.div`
  display: flex;
  flex: 1;
  flex-direction: row;
`;

const RightPanel = styled.div`
  display: flex;
  flex-direction: column;
  min-width: 200px;
`;

function App() {
  // State
  const [selectedSessionId, setSelectedSessionId] = useState('');
  const [selectedFrameId, setSelectedFrameId] = useState(0);
  const [selectedEntity, setSelectedEntity] = useState<Entity | undefined>();
  const [connected, setConnected] = useState(false);
  const [framesReceived, setFramesReceived] = useState(0);

  // Websocket
  useEffect(() => {
    const ws = new WebSocket(SERVER_ADDRESS);
    ws.onopen = () => {
      setConnected(true);
      ws.send(JSON.stringify({ command: Command.Init }));
    };
    ws.onclose = () => {
      setConnected(false);
      setFramesReceived(0);
    };
    ws.onmessage = createOnMessage(setSelectedSessionId, setFramesReceived);
  }, [setSelectedSessionId, setFramesReceived]);

  // Database
  const sessions = useLiveQuery(() => db.sessions.toArray()) ?? [];
  const frames =
    useLiveQuery(
      () =>
        db.frames
          .where('sessionId')
          .equals(selectedSessionId)
          .sortBy('frameNumber'),
      [selectedSessionId, framesReceived]
    ) ?? [];

  const entities = frames[selectedFrameId]?.entities ?? [];
  return (
    <OuterContainer>
      <TopContainer>
        <Suspense fallback={<LoadingScreen />}>
          <Viewer entities={entities} />
        </Suspense>
        <RightPanel>
          <SessionSelector
            sessions={sessions}
            setSelectedSessionId={setSelectedSessionId}
            connected={connected}
          />
          <EntityList
            entities={entities}
            setSelectedEntity={setSelectedEntity}
          />
          <Inspector entity={selectedEntity} />
        </RightPanel>
      </TopContainer>
      <Timeline
        maxFrames={frames.length}
        setSelectedFrameId={setSelectedFrameId}
        selectedFrameId={selectedFrameId}
      />
    </OuterContainer>
  );
}

function LoadingScreen(): React.ReactElement {
  return (
    <LoadingContainer>
      <h2>Loading..</h2>
    </LoadingContainer>
  );
}

const LoadingContainer = styled.div`
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  color: #fff;
`;

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
