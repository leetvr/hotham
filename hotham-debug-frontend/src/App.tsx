import React, { useEffect, useState } from 'react';
import { withTheme } from '@rjsf/core';
import { Theme as MaterialUITheme } from '@rjsf/material-ui';
import './App.css';
import { JSONSchema7 } from 'json-schema';
const SERVER_IP = 'localhost';
const ws = new WebSocket(`ws://${SERVER_IP}:8080`);

const Form = withTheme<Record<any, string>>(MaterialUITheme);

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

function update(editable: Record<string, any>) {
  ws.send(JSON.stringify({ Data: { editable } }));
}

function Container(props: { children: JSX.Element }): JSX.Element {
  return <div>{props.children}</div>;
}

function App() {
  const [editableData, setEditableData] = useState<
    Record<string, any> | undefined
  >();
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
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
      const message: Message = JSON.parse(m.data);
      if (message.Data) {
        if (message.Data.editable) {
          console.log('Received  data!');
          setEditableData(message.Data.editable);
        }
        if (message.Data.non_editable) {
          setNonEditableData(message.Data.non_editable);
        }
      }
      if (message.Init) {
        setSchema(message.Init.schema);
        setEditableData(message.Init.data.editable);
        setNonEditableData(message.Init.data.non_editable);
      }
      if (message.Error) {
        setError(error);
      }
    };
  });

  if (!schema || !editableData) {
    return (
      <Container>
        <h1>Loading..</h1>
      </Container>
    );
  }

  return (
    <Container>
      <>
        <h1>{error}</h1>
        <Form
          schema={schema.editable}
          formData={editableData}
          noHtml5Validate
          liveValidate
          onChange={({ formData, errors }) => {
            setEditableData(formData);
            if (!errors.length) {
              update(formData);
            }
          }}
        />
      </>
    </Container>
  );
}

export default App;
