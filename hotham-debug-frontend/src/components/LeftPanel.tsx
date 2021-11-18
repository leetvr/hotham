import React, { Suspense } from 'react';
import styled from 'styled-components';
import { Timeline } from './Timeline';
import { Viewer } from './Viewer';

const Container = styled.div`
  display: flex;
  flex: 3;
  flex-direction: column;
  position: relative;
`;

export function LeftPanel(): JSX.Element {
  return (
    <Container>
      <Suspense fallback={null}>
        <Viewer />
      </Suspense>
      <Timeline />
    </Container>
  );
}
