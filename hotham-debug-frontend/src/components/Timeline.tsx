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
  if (maxFrames === 0) {
    return (
      <OuterContainer>
        <h2>No frames available</h2>
        <Scrubber value={0} min={0} max={0} />
      </OuterContainer>
    );
  }

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
