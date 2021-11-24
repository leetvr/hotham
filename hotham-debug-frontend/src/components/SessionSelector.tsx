import React, { useEffect, useMemo, useState } from 'react';
import styled from 'styled-components';
import Tree from '@naisutech/react-tree';
import { Entity } from '../App';

const Container = styled.div`
  display: flex;
  flex: 1;
  flex-direction: column;
  color: white;
  overflow: none;
  overflow: hidden;
`;

interface Props {
  connected: boolean;
  sessionId: number;
  setSessionId: (n: number) => void;
}

export function SessionSelector(props: Props): JSX.Element {
  const { connected, sessionId, setSessionId } = props;

  if (connected) {
    <Container>
      <h2>Session in progress..</h2>
    </Container>;
  }

  const [sessions, setSessions] = useState<number[]>([]);
  useEffect(() => {
    const s = localStorage.getItem('sessions');
    if (!s) return;
    setSessions(JSON.parse(s));
  }, [connected]);

  const nodes = sessions.map((s) => ({
    id: s,
    parentId: null,
    label: s.toString(),
  }));

  return (
    <Container>
      <h2>Previous sessions</h2>
      <Tree
        nodes={nodes}
        onSelect={(n) => {
          if (!n.length) return;
          setTimeout(() => props.setSessionId(n[0] as number), 0);
        }}
      />
    </Container>
  );
}
