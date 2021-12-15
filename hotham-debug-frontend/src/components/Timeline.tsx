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
  setSelectedFrameId: (n: number) => void;
  selectedFrameId: number;
  maxFrames: number;
}

export function Timeline({
  selectedFrameId,
  setSelectedFrameId,
  maxFrames,
}: Props): JSX.Element {
  return (
    <OuterContainer>
      <h2>
        Frame {selectedFrameId + 1} / {maxFrames}
      </h2>
      <Scrubber
        min={1}
        max={maxFrames}
        value={selectedFrameId}
        onScrubChange={(c) => setSelectedFrameId(Math.round(c - 1))}
      />
    </OuterContainer>
  );
}
