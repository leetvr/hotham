import React from 'react';
import styled from 'styled-components';
import { Collider, Entity, Transform } from '../App';
import { Timeline } from './Timeline';

const Container = styled.div`
  display: flex;
  flex: 1;
  padding: 10px;
  background-color: #2d3439;
  flex-direction: column;
  color: #fff;
`;

const Indent = styled.div`
  padding-left: 10px;
`;

interface Props {
  entity?: Entity;
}

function TransformInspector({ t }: { t?: Transform }): JSX.Element | null {
  if (!t) return null;
  return (
    <Indent>
      <strong>transform: </strong>
      <Indent>
        <strong>translation: </strong>
        <br />
        <Indent>
          <strong>x:</strong> {t.translation[0]} <strong>y:</strong>{' '}
          {t.translation[1]} <strong>z:</strong> {t.translation[2]}
        </Indent>
        <strong>rotation: </strong>
        <br />
        <Indent>
          <strong>x:</strong> {t.rotation[0]} <strong>y:</strong>{' '}
          {t.rotation[1]} <strong>z:</strong> {t.rotation[2]}
        </Indent>
        <strong>scale: </strong>
        <br />
        <Indent>
          <strong>x:</strong> {t.scale[0]} <strong>y:</strong> {t.scale[1]}{' '}
          <strong>z:</strong> {t.scale[2]}
        </Indent>
      </Indent>
    </Indent>
  );
}

function ColliderInspector({ c }: { c?: Collider }): JSX.Element | null {
  if (!c) return <strong>No collider</strong>;
  return (
    <Indent>
      <strong>collider: </strong>
      <Indent>
        <strong>type:</strong> {c.colliderType}
        <br />
        <strong>geometry:</strong> {c.geometry}
      </Indent>
    </Indent>
  );
}

export function Inspector({ entity }: Props): JSX.Element {
  if (!entity)
    return (
      <Container>
        <h2>Inspector</h2>
        No entity selected.
      </Container>
    );

  return (
    <Container>
      <h2>Inspector</h2>
      <span>
        <strong>id: </strong> {entity.id}
      </span>
      <span>
        <strong>name: </strong> {entity.name} <br />
      </span>
      <TransformInspector t={entity.transform} />
      <ColliderInspector c={entity.collider} />
    </Container>
  );
}
