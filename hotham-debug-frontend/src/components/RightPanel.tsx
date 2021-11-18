import React from 'react';
import styled from 'styled-components';
import { Explorer } from './Explorer';
import { Inspector } from './Inspector';

const Container = styled.div`
  display: flex;
  flex: 1;
  flex-direction: column;
`;

export function RightPanel(): JSX.Element {
  return (
    <Container>
      <Explorer />
      <Inspector />
    </Container>
  );
}
