import React, { useEffect, useState } from 'react';
import { JSONSchema7 } from 'json-schema';
import styled from 'styled-components';
import { LeftPanel } from './components/LeftPanel';
import { RightPanel } from './components/RightPanel';
const SERVER_IP = 'localhost';
const ws = new WebSocket(`ws://${SERVER_IP}:8080`);

enum Command {
  Reset,
  Init,
}

interface Data {
  editable?: Record<string, any>;
  non_editable?: Record<string, any>;
}

interface InitData {
  data: Data;
  schema: Schema;
}

interface Message {
  Data: Data;
  Command: Command;
  Init: InitData;
  Error: string;
}

interface Schema {
  editable: JSONSchema7;
  non_editable: JSONSchema7;
}

type Entities = Record<number, Entity>;
interface Frame {
  id: number;
  entities: Entities;
}

function update(editable: Record<string, any>) {
  ws.send(JSON.stringify({ Data: { editable } }));
}

const Container = styled.div`
  display: flex;
  flex: 1;
  flex-direction: row;
  height: 100vh;
  width: 100vw;
`;

function App() {
  const [editableData, setEditableData] = useState<
    Record<string, any> | undefined
  >();
  const [noneditableData, setNonEditableData] = useState<
    Record<string, any> | undefined
  >();
  const [error, setError] = useState<string | undefined>();
  const [schema, setSchema] = useState<Schema | undefined>();
  useEffect(() => {
    ws.onopen = () => {
      ws.send(JSON.stringify({ Command: Command.Init }));
    };
  });
  useEffect(() => {
    ws.onmessage = (m) => {
      // const message: Message = JSON.parse(m.data);
      // if (messagej.Data) {
      //   if (message.Data.editable) {
      //     console.log('Received  data!');
      //     setEditableData(message.Data.editable);
      //   }
      //   if (message.Data.non_editable) {
      //     const deltaTime = lastUpdate - new Date().getTime();
      //     if (deltaTime > 500) {
      //       setNonEditableData(message.Data.non_editable);
      //       lastUpdate = new Date().getTime();
      //     }
      //   }
      // }
      // if (message.Init) {
      //   setSchema(message.Init.schema);
      //   setEditableData(message.Init.data.editable);
      //   setNonEditableData(message.Init.data.non_editable);
      // }
      // if (message.Error) {
      //   setError(error);
      // }
    };
  });

  const [frame, setFrame] = useState(0);
  const maxFrames = 10;

  const frames: Frame[] = [
    {
      id: 0,
      entities: {
        0: {
          name: 'Environment',
          id: 0,
          mesh: 'Environment',
          material: 'Rough',
          transform: {
            translation: [0, 0, -1],
            rotation: [0, 0, 0],
            scale: [1, 1, 1],
          },
          collider: {
            colliderType: 'cube',
            geometry: [1, 1, 1],
          },
        },
        1: { name: 'Empty', id: 1 },
      },
    },
    {
      id: 1,
      entities: {
        0: {
          name: 'Environment',
          id: 0,
          mesh: 'Environment',
          material: 'Rough',
          transform: {
            translation: [0, 0, -1.1],
            rotation: [0, 0, 0],
            scale: [1, 1, 1],
          },
          collider: {
            colliderType: 'cube',
            geometry: [1, 1, 1],
          },
        },
      },
    },
  ];

  const entities = frames[frame].entities;

  return (
    <Container>
      <LeftPanel
        entities={entities}
        frame={frame}
        setFrame={setFrame}
        maxFrames={frames.length}
      />
      <RightPanel entities={entities} />
    </Container>
  );
}

export interface Transform {
  translation: [number, number, number];
  rotation: [number, number, number];
  scale: [number, number, number];
}

export interface Entity {
  id: number;
  name: string;
  mesh?: string;
  material?: string;
  transform?: Transform;
  collider?: {
    colliderType: 'cube' | 'cylinder';
    geometry: number[];
  };
}

export default App;
