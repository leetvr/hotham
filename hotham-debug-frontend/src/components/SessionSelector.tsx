import * as React from 'react';
import styled from 'styled-components';
import Tree from '@naisutech/react-tree';
import { Session } from '../App';

const Container = styled.div`
  display: flex;
  flex: 1;
  flex-direction: column;
  color: white;
  overflow: hidden;
  padding: 5px;
`;

interface Props {
  sessions: Session[];
  setSelectedSessionId: (id: string) => void;
  connected: boolean;
}

export function SessionSelector(props: Props): JSX.Element {
  const { sessions, connected, setSelectedSessionId } = props;
  if (connected) {
    return (
      <Container>
        <h2>Connected to device</h2>
      </Container>
    );
  }

  const nodes = sessions.map((s) => ({
    id: s.id,
    parentId: null,
    label: s.timestamp.toLocaleString(),
  }));

  return (
    <Container>
      <h2>Previous sessions</h2>
      <Tree
        nodes={nodes}
        onSelect={(n) => {
          if (!n.length) return;
          setTimeout(() => setSelectedSessionId(String(n[0])), 0);
        }}
      />
    </Container>
  );
}
