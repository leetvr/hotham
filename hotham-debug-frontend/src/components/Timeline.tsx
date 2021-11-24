import styled from 'styled-components';
import { Scrubber } from 'react-scrubber';
import 'react-scrubber/lib/scrubber.css';

const OuterContainer = styled.div`
  display: flex;
  flex-direction: column;
  padding: 10px;
  color: #fff;
  min-height: 100px;
`;

const Container = styled.div`
  display: flex;
  flex: 1;
  align-items: center;
  flex-direction: row;
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
      <h2>
        Frame {frame} / {maxFrames}
      </h2>
      <Scrubber
        min={0}
        max={maxFrames}
        value={frame}
        onScrubChange={(c) => setFrame(Math.round(c))}
      />
    </OuterContainer>
  );
}
