import Dexie, { Table } from 'dexie';
import { Frame, Session } from './App';

const DATABASE_NAME = 'hotham-debug-frontend';
const schema = {
  frames: 'id, sessionId, frameNumber',
  sessions: 'id',
};

export class Database extends Dexie {
  frames!: Table<Frame>;
  sessions!: Table<Session>;

  constructor() {
    super(DATABASE_NAME);
    this.version(1).stores(schema);
  }
}

export const db = new Database();
