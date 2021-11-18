import React, { useState } from 'react';
import styled from 'styled-components';
import { Explorer } from './Explorer';
import { Inspector } from './Inspector';

const Container = styled.div`
  display: flex;
  flex: 1;
  flex-direction: column;
`;

export interface Entity {
  id: number;
}

export function RightPanel(): JSX.Element {
  const entities: Record<number, Entity> = {
    0: { id: 0 },
    1: { id: 1 },
  };
  const [selectedEntityId, selectEntityId] = useState<number | undefined>();
  const selectedEntity =
    selectedEntityId !== undefined ? entities[selectedEntityId] : null;
  return (
    <Container>
      <Explorer selectEntityId={selectEntityId} />
      <Inspector entity={selectedEntity} />
    </Container>
  );
}
