import React, { useEffect, useState } from 'react';
import './App.css';
const ws = new WebSocket('ws://localhost:8080');

interface Test {
  count: number;
}

function resetCount() {
  ws.send(JSON.stringify({ count: 0 }));
}

function App() {
  const [count, setCount] = useState(0);
  useEffect(() => {
    ws.onmessage = (m) => {
      const data: Test = JSON.parse(m.data);
      setCount(data.count);
    };
  });
  return (
    <div className="App">
      <header className="App-header">
        <h1>{count}</h1>
        <div className="button" onClick={resetCount}>
          Reset
        </div>
      </header>
    </div>
  );
}

export default App;
