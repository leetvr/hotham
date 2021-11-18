import React from 'react';
import styled from 'styled-components';
import { DisplayOptions } from './Viewer';

const Container = styled.div`
  display: flex;
  flex-direction: row;
  flex-grow: 0;
`;

interface OptionProps {
  selected?: boolean;
}

const Option = styled.div<OptionProps>`
  padding: 5px 10px;
  background-color: ${(p) => (p.selected ? '#ccc' : '#eee')};
  border-radius: 2px;
  margin: 5px;
`;

const OptionGroup = styled.div`
  padding: 5px;
  background-color: #aaa;
  display: flex;
  flex-direction: column;
  margin: 5px;
`;
const Options = styled.div`
  display: flex;
  flex-direction: row;
`;

interface Props {
  displays: DisplayOptions;
  setDisplays: React.Dispatch<React.SetStateAction<DisplayOptions>>;
}

export function ViewOptions({ displays, setDisplays }: Props): JSX.Element {
  return (
    <Container>
      <OptionGroup>
        <span>Display options:</span>
        <Options>
          <Option
            selected={displays.models}
            onClick={() =>
              setDisplays((d) => ({ ...d, models: !displays.models }))
            }
          >
            Models
          </Option>
          <Option
            selected={displays.physics}
            onClick={() =>
              setDisplays((d) => ({ ...d, physics: !displays.physics }))
            }
          >
            Physics
          </Option>
        </Options>
      </OptionGroup>
    </Container>
  );
}
