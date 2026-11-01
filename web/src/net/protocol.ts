export const NET_STATE_FLOATS = 64;

export interface PlayerInfo {
  id: number;
  slot: number;
  name: string;
}

export interface StateInfo {
  player_id: number;
  seq: number;
  tick: number;
  state: number[];
}

export interface WelcomeMessage {
  type: "welcome";
  player_id: number;
  slot: number;
  room: string;
  car: string;
  track: string;
  max_players: number;
  players: PlayerInfo[];
  states: StateInfo[];
}

export interface RosterMessage {
  type: "roster";
  players: PlayerInfo[];
}

export interface StateMessage {
  type: "state";
  player_id: number;
  seq: number;
  tick: number;
  state: number[];
}

export interface PlayerLeftMessage {
  type: "player_left";
  player_id: number;
}

export interface ErrorMessage {
  type: "error";
  message: string;
}

export interface PongMessage {
  type: "pong";
}

export type ServerMessage =
  | WelcomeMessage
  | RosterMessage
  | StateMessage
  | PlayerLeftMessage
  | ErrorMessage
  | PongMessage;

export function validState(state: number[]): boolean {
  return state.length === NET_STATE_FLOATS && state.every(Number.isFinite);
}

export function parseServerMessage(raw: string): ServerMessage | null {
  let message: unknown;
  try {
    message = JSON.parse(raw);
  } catch {
    return null;
  }
  if (!message || typeof message !== "object" || !("type" in message)) {
    return null;
  }
  const type = (message as { type?: unknown }).type;
  switch (type) {
    case "welcome":
    case "roster":
    case "state":
    case "player_left":
    case "error":
    case "pong":
      return message as ServerMessage;
    default:
      return null;
  }
}
