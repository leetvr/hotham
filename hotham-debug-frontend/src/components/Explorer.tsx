import React from 'react';
import styled from 'styled-components';
import Tree from '@naisutech/react-tree';
import { Entity } from '../App';

const Container = styled.div`
  display: flex;
  flex: 1;
`;

interface Props {
  entities: Record<number, Entity>;
  selectEntityId: React.Dispatch<React.SetStateAction<number | undefined>>;
}

export function Explorer(props: Props): JSX.Element {
  const nodes = getNodes(props.entities);
  return (
    <Container>
      <Tree
        nodes={nodes}
        onSelect={(n) => {
          if (!n.length) return;
          setTimeout(() => props.selectEntityId(n[0] as number | undefined), 0);
        }}
      />
    </Container>
  );
}
function getNodes(entities: Record<number, Entity>) {
  return Object.values(entities).map((e) => ({
    id: e.id,
    parentId: null,
    label: e.name,
  }));
}
