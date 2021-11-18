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
  const nodes = [
    {
      id: 0,
      parentId: null,
      label: 'Test',
    },
    {
      id: 1,
      parentId: 0,
      label: 'Test Child',
    },
  ];
  return (
    <Container>
      <Tree
        nodes={nodes}
        onSelect={(n) =>
          setTimeout(() => props.selectEntityId(n[0] as number | undefined), 0)
        }
      />
    </Container>
  );
}
