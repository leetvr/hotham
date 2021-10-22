import React, { useEffect, useState } from 'react';
import Form from '@rjsf/core';
import './App.css';
import { JSONSchema7 } from 'json-schema';
const ws = new WebSocket('ws://localhost:8080');

interface Test {
  count: number;
}

enum Command {
  Reset,
  GetSchema,
}

interface Message {
  Data: Test;
  Command: Command;
  Schema: string;
}

function update(val: Test) {
  ws.send(JSON.stringify({ Data: val }));
}

function Container(props: { children: JSX.Element }): JSX.Element {
  return (
    <div className="App">
      <header className="App-header">{props.children}</header>
    </div>
  );
}

function App() {
  const [data, setData] = useState<Test>({ count: 0 });
  const [schema, setSchema] = useState<JSONSchema7 | undefined>();
  useEffect(() => {
    ws.onopen = () => {
      ws.send(JSON.stringify({ Command: Command.GetSchema }));
    };
  });
  useEffect(() => {
    ws.onmessage = (m) => {
      const message: Message = JSON.parse(m.data);
      if (message.Data) {
        setData(message.Data);
      }
      if (message.Schema) {
        const schema: JSONSchema7 = JSON.parse(message.Schema);
        setSchema(schema);
      }
    };
  });

  if (!schema) {
    return (
      <Container>
        <h1>Loading..</h1>
      </Container>
    );
  }

  return (
    <Container>
      <Form<Test>
        formData={data}
        schema={schema}
        onChange={(e) => update(e.formData)}
      />
    </Container>
  );
}

export default App;
