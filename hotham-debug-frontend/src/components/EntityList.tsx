import React from 'react';
import styled from 'styled-components';
import Tree from '@naisutech/react-tree';
import { Entity } from '../App';

const Container = styled.div`
  display: flex;
  flex: 1;
  color: #fff;
  overflow: hidden;
  flex-direction: column;
  padding: 5px;
`;

interface Props {
  entities: Entity[];
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
function getNodes(entities: Entity[]) {
  return entities.map((e, i) => ({
    id: i,
    parentId: null,
    label: `${e.name} (${e.entityId})`,
  }));
}
