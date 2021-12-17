import { useLiveQuery } from 'dexie-react-hooks';
import React, { Suspense, useState } from 'react';
import styled from 'styled-components';
import { EntityList } from './components/EntityList';
import { Inspector } from './components/Inspector';
import { SessionSelector } from './components/SessionSelector';
import { Timeline } from './components/Timeline';
import { Viewer } from './components/Viewer';
import { db } from './db';
import { ServerState, useServerConnector } from './ws';

const OuterContainer = styled.div`
  display: flex;
  flex: 1;
  height: 100vh;
  width: 100vw;
  flex-direction: row;
  background-color: #2d3439;
`;

const LeftContainer = styled.div`
  display: flex;
  flex: 1;
  flex-direction: column;
`;

const RightPanel = styled.div`
  display: flex;
  flex-direction: column;
  width: 20vw;
  height: 100vh;
`;

function App() {
  // State
  const [selectedSessionId, setSelectedSessionId] = useState('');
  const [selectedFrameId, setSelectedFrameId] = useState(0);
  const [selectedEntity, setSelectedEntity] = useState<Entity | undefined>();

  // Websocket
  const { framesReceived, server, state } = useServerConnector();
  const connected = state === ServerState.CONNECTED;

  // Database
  const sessions =
    useLiveQuery(() => db.sessions.orderBy('timestamp').reverse().toArray()) ??
    [];
  const framesFromDB =
    useLiveQuery(() => {
      if (connected) return [];
      return db.frames
        .where('sessionId')
        .equals(selectedSessionId)
        .sortBy('frameNumber');
    }, [selectedSessionId, connected]) ?? [];

  // Frames and Entities for UI
  const frames = connected ? server.frames : framesFromDB;
  const entities = frames[selectedFrameId]?.entities ?? [];
  const maxFrames = connected ? framesReceived : frames.length;

  return (
    <OuterContainer>
      <LeftContainer>
        <Suspense fallback={<LoadingScreen />}>
          <Viewer entities={entities} />
        </Suspense>
        <Timeline
          maxFrames={maxFrames}
          setSelectedFrameId={setSelectedFrameId}
          selectedFrameId={selectedFrameId}
        />
      </LeftContainer>
      <RightPanel>
        <SessionSelector
          sessions={sessions}
          setSelectedSessionId={setSelectedSessionId}
          connected={connected}
        />
        <EntityList entities={entities} setSelectedEntity={setSelectedEntity} />
        <Inspector entity={selectedEntity} />
      </RightPanel>
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

export type Vector3 = [number, number, number];

export enum Command {
  Reset,
  Init,
}

export interface InitData {
  firstFrame: Frame;
  sessionId: string;
}

export interface Message {
  frames?: Frame[];
  command?: Command;
  init?: InitData;
  error?: string;
}

export interface Frame {
  id: string;
  frameNumber: number;
  entities: Entity[];
  sessionId: string;
}

export interface Session {
  id: string;
  timestamp: Date;
}

export interface Transform {
  translation: Vector3;
  rotation: Vector3;
  scale: Vector3;
}

export interface Collider {
  colliderType: 'cube' | 'cylinder';
  geometry: number[];
  translation: Vector3;
  rotation: Vector3;
}

export interface Entity {
  id: string;
  entityId: number;
  name: string;
  transform?: Transform;
  collider?: Collider;
}

export default App;
