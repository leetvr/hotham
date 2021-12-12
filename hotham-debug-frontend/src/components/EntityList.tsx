import React from 'react';
import styled from 'styled-components';
import Tree from '@naisutech/react-tree';
import { Entities, Entity } from '../App';

const Container = styled.div`
  display: flex;
  flex: 1;
  color: #fff;
  overflow: hidden;
  flex-direction: column;
`;

interface Props {
  entities: Entities;
  setSelectedEntity: (e: Entity) => void;
}

export function EntityList(props: Props): JSX.Element {
  const { entities, setSelectedEntity } = props;
  const nodes = getNodes(entities);
  return (
    <Container>
      <h2>Entities</h2>
      <Tree
        nodes={nodes}
        onSelect={(n) => {
          if (!n.length) return;
          const index = Number(n[0]);
          const selectedEntity = entities[index];
          setTimeout(() => setSelectedEntity(selectedEntity), 0);
        }}
      />
    </Container>
  );
}
function getNodes(entities: Entities) {
  return Object.values(entities).map((e) => ({
    id: e.id,
    parentId: null,
    label: e.name,
  }));
}
