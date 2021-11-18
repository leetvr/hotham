import React from 'react';
import styled from 'styled-components';
import { Entity, Transform } from '../App';

const Container = styled.div`
  display: flex;
  flex: 1;
  padding: 10px;
  background-color: #2d3439;
  flex-direction: column;
`;

const Indent = styled.div`
  padding-left: 10px;
`;

interface Props {
  entity: Entity | null;
}

function TransformInspector({ t }: { t?: Transform }): JSX.Element | null {
  if (!t) return null;
  return (
    <Indent>
      <strong>translation: </strong>
      <br />
      <Indent>
        <strong>x:</strong> {t.translation[0]} <strong>y:</strong>{' '}
        {t.translation[1]} <strong>z:</strong> {t.translation[2]}
      </Indent>
    </Indent>
  );
}

export function Inspector({ entity }: Props): JSX.Element {
  if (!entity) return <Container>No entity selected.</Container>;

  return (
    <Container>
      <span>
        <strong>id: </strong> {entity.id}
      </span>
      <span>
        <strong>name: </strong> {entity.name} <br />
      </span>
      <TransformInspector t={entity.transform} />
    </Container>
  );
}
