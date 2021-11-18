import React, { VoidFunctionComponent } from 'react';
import styled from 'styled-components';

const OuterContainer = styled.div`
  display: flex;
  flex: 1;
  flex-direction: column;
`;

const Container = styled.div`
  display: flex;
  flex: 1;
  align-items: center;
  flex-direction: row;
  padding: 10px;
`;

const TimelineItem = styled.div<TimelineItemProps>`
  display: flex;
  height: 50px;
  width: 50px;
  background-color: #bbb;
  border-color: ${(p) => (p.selected ? '#eee' : '#bbb')};
  border-width: 5px;
  border-style: solid;
  border-radius: 50%;
  align-items: center;
  justify-content: center;
  font-weight: ${(p) => (p.selected ? 'bold' : '')};
`;

const Spacer = styled.div`
  display: flex;
  height: 10px;
  width: 10px;
  background-color: #bbb;
  zindex: -1;
`;

interface TimelineItemProps {
  selected?: boolean;
}

interface Props {
  setFrame: (n: number) => void;
  frame: number;
  maxFrames: number;
}

function getFrames(
  frame: number,
  setFrame: (n: number) => void,
  maxFrames: number
): JSX.Element[] {
  const elements = [];
  for (let i = 0; i < maxFrames; i++) {
    elements.push(
      <TimelineItem selected={i === frame} onClick={() => setFrame(i)} key={i}>
        {i}
      </TimelineItem>
    );
    if (i < maxFrames - 1) {
      elements.push(<Spacer />);
    }
  }

  return elements;
}

export function Timeline({ frame, setFrame, maxFrames }: Props): JSX.Element {
  return (
    <OuterContainer>
      <Container>{getFrames(frame, setFrame, maxFrames)}</Container>
    </OuterContainer>
  );
}
