import { Message, Command, Frame } from './App';
import * as React from 'react';
import { db } from './db';
import { useMemo, useState } from 'react';

export enum ServerState {
  CONNECTED,
  CONNECTING,
  DISCONNECTED,
}

export const SERVER_ADDRESS = `ws://localhost:8000`;
class ServerConnector {
  setServerState: React.Dispatch<React.SetStateAction<ServerState>>;
  setFramesReceived: React.Dispatch<React.SetStateAction<number>>;
  ws: WebSocket | undefined;
  frames: Frame[];
  internalState: ServerState;

  constructor(
    setServerState: React.Dispatch<React.SetStateAction<ServerState>>,
    setFramesReceived: React.Dispatch<React.SetStateAction<number>>
  ) {
    console.log('Creating server');
    this.setServerState = setServerState;
    this.setFramesReceived = setFramesReceived;
    this.internalState = ServerState.DISCONNECTED;
    this.frames = [];
    this.connect();
  }

  connect() {
    if (this.internalState !== ServerState.DISCONNECTED) return;

    this.internalState = ServerState.CONNECTING;
    console.log(new Date(), 'Connecting..');
    const ws = new WebSocket(SERVER_ADDRESS);
    ws.onopen = this.onOpen;
    ws.onmessage = this.onMessage;
    ws.onclose = this.onClose;
    this.ws = ws;
  }

  onOpen = () => {
    this.frames = [];
    this.setServerState(ServerState.CONNECTED);
    this.internalState = ServerState.CONNECTED;
    console.log(new Date(), 'Connected. Sending INIT');
    this.ws!.send(JSON.stringify({ command: Command.Init }));
  };

  onMessage = (e: MessageEvent) => {
    const { init, frames }: Message = JSON.parse(e.data);
    if (frames) {
      for (let frame of frames) {
        this.frames.push(frame);
      }
      this.setFramesReceived(this.frames.length);
      return;
    }

    if (init) {
      this.frames.push(init.firstFrame);
      this.setFramesReceived(this.frames.length);

      db.sessions
        .put({ id: init.sessionId, timestamp: new Date() })
        .catch(console.error);
    }
  };

  onClose = () => {
    console.log(new Date(), 'onClose called', this.internalState);
    if (this.internalState === ServerState.CONNECTED) {
      this.ws = undefined;
      db.frames.bulkPut(this.frames);
      this.frames = [];
    }

    this.internalState = ServerState.DISCONNECTED;
    this.setServerState(ServerState.DISCONNECTED);
    console.log(new Date(), 'Disconnected. Attempting to reconnect..');
    setTimeout(() => {
      console.log(new Date(), 'Reconnecting..');
      this.connect();
    }, 1000);
  };
}

//   ws.onerror = (e) => {
//     if (connected) return;
//     console.warn('Error connecting to server, retrying');
//     setTimeout(
//       () => connectToServer(setConnected, setFramesFromServer, connected),
//       1000
//     );
//   };

//   return ws;
// }

interface Hook {
  framesReceived: number;
  server: ServerConnector;
  state: ServerState;
}

export function useServerConnector(): Hook {
  const [serverState, setServerState] = useState<ServerState>(
    ServerState.DISCONNECTED
  );
  const [framesReceived, setFramesReceived] = useState<number>(0);
  const server = useMemo(() => {
    return new ServerConnector(setServerState, setFramesReceived);
  }, []);

  // React.useMemo(() => {
  //   console.log('Creating websocket');
  //   const ws = new WebSocket(SERVER_ADDRESS);
  //   ws.onopen = () => {
  //     console.log(new Date(), 'Opened');
  //     ws.send(JSON.stringify({ command: Command.Init }));
  //   };
  //   ws.onclose = () => {
  //     console.log(new Date(), 'Closed');
  //   };
  // }, []);

  return { framesReceived, server, state: serverState };
}
