import React from 'react';
import { Entities, Frame } from '../App';

interface AppState {
  connected: boolean;
  entities: Entities;
  frames: Frame[];
}

// TODO
// export const AppStateContext = React.createContext<AppState>({});
