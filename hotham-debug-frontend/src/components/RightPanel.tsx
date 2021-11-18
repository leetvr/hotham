import React, { useState } from 'react';
import styled from 'styled-components';
import { Entity } from '../App';
import { Explorer } from './Explorer';
import { Inspector } from './Inspector';

const Container = styled.div`
  display: flex;
  flex: 1;
  flex-direction: column;
`;

interface Props {
  entities: Record<number, Entity>;
}

export function RightPanel({ entities }: Props): JSX.Element {
  const [selectedEntityId, selectEntityId] = useState<number | undefined>();
  const selectedEntity =
    selectedEntityId !== undefined ? entities[selectedEntityId] : null;
  return (
    <Container>
      <Explorer entities={entities} selectEntityId={selectEntityId} />
      <Inspector entity={selectedEntity} />
    </Container>
  );
}
