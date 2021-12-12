import 'fake-indexeddb/auto';
import { act, render, waitFor, within } from '@testing-library/react';
import App, { Frame, SERVER_ADDRESS } from './App';
import userEvent from '@testing-library/user-event';
import { db } from './db';
import WS from 'jest-websocket-mock';

function MockScrubber(props: {
  min: number;
  max: number;
  value: number;
  onScrubChange: (n: number) => void;
}): JSX.Element {
  const { max, onScrubChange } = props;
  return (
    <div
      data-testid="scrubber"
      onClick={() => {
        onScrubChange(max - 1);
      }}
    />
  );
}

jest.mock('react-scrubber', () => ({
  Scrubber: MockScrubber,
}));

jest.mock('ws', () => {});

const stubFrames: Frame[] = [
  {
    sessionId: 0,
    id: 0,
    entities: {
      0: {
        id: 0,
        name: 'Test Entity 1',
        transform: {
          translation: [0, 0, 0],
          rotation: [0, 0, 0],
          scale: [1, 1, 1],
        },
        collider: {
          colliderType: 'cube',
          geometry: [1, 2, 3],
        },
      },
      1: {
        id: 1,
        name: 'Test Entity 2',
      },
    },
  },
  {
    sessionId: 0,
    id: 1,
    entities: {
      0: {
        id: 0,
        name: 'Test Entity 3',
      },
    },
  },
];

beforeAll(async () => {
  await db.sessions.bulkAdd([
    { id: 0, timestamp: new Date() },
    { id: 1, timestamp: new Date() },
  ]);

  await db.frames.bulkAdd(stubFrames);
});

const DATE_REGEX = new RegExp(/\d{1,2}\/\d{1,2}\/\d{4}/);

test('renders a list of sessions when not connected', async () => {
  const { getByText } = render(<App />);
  const sessionContainer = getByText(/Previous sessions/i).parentElement;
  const sessionDate = await within(sessionContainer!).findAllByText(DATE_REGEX);
  expect(sessionDate).toHaveLength(2);
});

test('does not show sessions when connected', async () => {
  const server = new WS(SERVER_ADDRESS);
  const { getByText } = render(<App />);
  await server.connected;

  expect(getByText(/Connected to device/i)).toBeInTheDocument();

  WS.clean();
});

test('sends an INIT message when first connected', async () => {
  const server = new WS(SERVER_ADDRESS, { jsonProtocol: true });
  render(<App />);
  await server.connected;

  await expect(server).toReceiveMessage({ Command: 1 });

  WS.clean();
});

test('the entity window gets populated with the first frame', async () => {});

test('clicking on a session changes the selected session', async () => {
  const { getByText } = render(<App />);
  const sessionContainer = getByText(/Previous sessions/i).parentElement;
  const session = (
    await within(sessionContainer!).findAllByText(DATE_REGEX)
  )[0];

  act(() => {
    userEvent.click(session);
  });

  // Ensure the entities have loaded.
  const entitiesContainer = getByText(/Entities/i).parentElement;
  expect(
    await within(entitiesContainer!).findByText('Test Entity 1')
  ).toBeInTheDocument();
  expect(
    await within(entitiesContainer!).findByText('Test Entity 2')
  ).toBeInTheDocument();
});

test('clicking on an entity shows details about that entity', async () => {
  const { getByText, getByRole } = render(<App />);
  const sessionContainer = getByText(/Previous sessions/i).parentElement;
  const session = (
    await within(sessionContainer!).findAllByText(DATE_REGEX)
  )[0];

  act(() => {
    userEvent.click(session);
  });

  // Ensure the entities are in the EntityList.
  const entitiesContainer = getByText(/Entities/i).parentElement;
  const entity = await within(entitiesContainer!).findByText('Test Entity 1');

  act(() => {
    userEvent.click(entity);
  });

  // Ensure the entity's properties are visible.
  const inspectorContainer = getByRole('heading', {
    name: 'Inspector',
  }).parentElement;

  await waitFor(() => {
    expect(inspectorContainer).toHaveTextContent('name: Test Entity 1');
    expect(inspectorContainer).toHaveTextContent('translation: x: 0 y: 0 z: 0');
    expect(inspectorContainer).toHaveTextContent('rotation: x: 0 y: 0 z: 0');
    expect(inspectorContainer).toHaveTextContent('scale: x: 1 y: 1 z: 1');
  });
});

test('clicking on the frame slider changes the current frame', async () => {
  const { getByText, getByTestId } = render(<App />);
  const sessionContainer = getByText(/Previous sessions/i).parentElement;
  const session = (
    await within(sessionContainer!).findAllByText(DATE_REGEX)
  )[0];

  act(() => {
    userEvent.click(session);
  });

  // Wait for the first session to load..
  const entitiesContainer = getByText(/Entities/i).parentElement;
  await within(entitiesContainer!).findByText('Test Entity 2');

  // Now, click on the scrubber so it loads the next frame..
  const timeline = getByTestId('scrubber');
  act(() => {
    userEvent.click(timeline);
  });

  const entity = await within(entitiesContainer!).findByText('Test Entity 3');
  expect(entity).toBeInTheDocument();
});
