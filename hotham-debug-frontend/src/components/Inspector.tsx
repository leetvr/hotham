import React from 'react';
import styled from 'styled-components';
import { Entity } from './RightPanel';

const Container = styled.div`
  display: flex;
  flex: 1;
  padding: 10px;
  background-color: #2d3439;
`;

interface Props {
  entity: Entity | null;
}
export function Inspector({ entity }: Props): JSX.Element {
  if (!entity) return <Container>No entity selected.</Container>;

  return (
    <Container>
      <div>
        <strong>id: </strong> {entity.id}
      </div>
    </Container>
  );
}
