import React, { useState } from 'react';
import styled from 'styled-components';
import { Entities, Entity } from '../App';
import { Explorer } from './Explorer';
import { Inspector } from './Inspector';
import { SessionSelector } from './SessionSelector';

const Container = styled.div`
  display: flex;
  flex: 1;
  flex-direction: column;
`;

interface Props {
  entities: Entities;
  connected: boolean;
  sessionId: number;
  setSessionId: (n: number) => void;
}

export function RightPanel({
  entities,
  connected,
  sessionId,
  setSessionId,
}: Props): JSX.Element {
  const [selectedEntityId, selectEntityId] = useState<number | undefined>();
  const selectedEntity =
    selectedEntityId !== undefined ? entities[selectedEntityId] : null;
  return (
    <Container>
      <SessionSelector
        connected={connected}
        setSessionId={setSessionId}
        sessionId={sessionId}
      />
      <Explorer entities={entities} selectEntityId={selectEntityId} />
      <Inspector entity={selectedEntity} />
    </Container>
  );
}
