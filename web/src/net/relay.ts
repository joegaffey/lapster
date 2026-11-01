import { loadRelayUrl } from "./room.ts";
import {
  parseServerMessage,
  type PlayerInfo,
  type ServerMessage,
  type StateMessage,
  type WelcomeMessage,
} from "./protocol.ts";

export interface RelayOptions {
  room: string;
  password: string;
  name: string;
  car: string;
  track: string;
  url?: string;
}

export class RelayClient {
  playerId = 0;
  slot = 0;
  room = "";
  maxPlayers = 4;
  players: PlayerInfo[] = [];
  onRoster: ((players: PlayerInfo[]) => void) | null = null;
  onState: ((message: StateMessage) => void) | null = null;
  onPlayerLeft: ((playerId: number) => void) | null = null;
  onError: ((message: string) => void) | null = null;
  onClose: ((event: CloseEvent) => void) | null = null;

  private readonly socket: WebSocket;
  private welcomeResolve: ((welcome: WelcomeMessage) => void) | null = null;
  private welcomeReject: ((error: Error) => void) | null = null;
  private welcomeTimer: ReturnType<typeof setTimeout> | null = null;
  private intentionalClose = false;

  constructor(private readonly options: RelayOptions) {
    this.socket = new WebSocket(options.url ?? resolveRelayUrl());
    this.socket.onopen = () => {
      this.socket.send(
        JSON.stringify({
          type: "join",
          room: options.room,
          password: options.password,
          name: options.name,
          car: options.car,
          track: options.track,
        }),
      );
    };
    this.socket.onmessage = (event) => this.handleMessage(event.data);
    this.socket.onerror = () => {
      this.fail(new Error("relay connection failed"));
    };
    this.socket.onclose = (event) => {
      this.fail(new Error("relay connection closed"));
      if (!this.intentionalClose) {
        this.onClose?.(event);
      }
    };
  }

  connect(): Promise<WelcomeMessage> {
    return new Promise((resolve, reject) => {
      this.welcomeResolve = resolve;
      this.welcomeReject = reject;
      this.welcomeTimer = setTimeout(() => {
        this.fail(new Error("relay join timed out"));
        this.close();
      }, 10_000);
    });
  }

  sendState(seq: number, tick: number, state: ArrayLike<number>): void {
    if (this.socket.readyState !== WebSocket.OPEN) {
      return;
    }
    this.socket.send(
      JSON.stringify({
        type: "state",
        seq,
        tick,
        state: Array.from(state, (value) => (Number.isFinite(value) ? value : 0)),
      }),
    );
  }

  sendPing(): void {
    if (this.socket.readyState === WebSocket.OPEN) {
      this.socket.send(JSON.stringify({ type: "ping" }));
    }
  }

  close(): void {
    this.intentionalClose = true;
    if (this.welcomeTimer !== null) {
      clearTimeout(this.welcomeTimer);
      this.welcomeTimer = null;
    }
    this.socket.close();
  }

  private handleMessage(data: unknown): void {
    if (typeof data !== "string") {
      return;
    }
    const message = parseServerMessage(data);
    if (!message) {
      return;
    }
    switch (message.type) {
      case "welcome":
        this.playerId = message.player_id;
        this.slot = message.slot;
        this.room = message.room;
        this.maxPlayers = message.max_players;
        this.players = message.players;
        if (this.welcomeTimer !== null) {
          clearTimeout(this.welcomeTimer);
          this.welcomeTimer = null;
        }
        this.welcomeResolve?.(message);
        this.welcomeResolve = null;
        this.welcomeReject = null;
        this.onRoster?.(message.players);
        break;
      case "roster":
        this.players = message.players;
        this.onRoster?.(message.players);
        break;
      case "state":
        this.onState?.(message);
        break;
      case "player_left":
        this.onPlayerLeft?.(message.player_id);
        break;
      case "error":
        this.fail(new Error(message.message));
        break;
      case "pong":
        break;
    }
  }

  private fail(error: Error): void {
    if (this.welcomeReject) {
      if (this.welcomeTimer !== null) {
        clearTimeout(this.welcomeTimer);
        this.welcomeTimer = null;
      }
      this.welcomeReject(error);
      this.welcomeReject = null;
      this.welcomeResolve = null;
      return;
    }
    this.onError?.(error.message);
  }
}

export function resolveRelayUrl(
  href: string | undefined = globalThis.location?.href,
): string {
  const base = new URL(href ?? "http://127.0.0.1/");
  const override = base.searchParams.get("ws") || loadRelayUrl();
  const url = new URL(override || "/ws", base);
  if (url.protocol === "http:") {
    url.protocol = "ws:";
  } else if (url.protocol === "https:") {
    url.protocol = "wss:";
  }
  return url.toString();
}

export type { ServerMessage, StateMessage, WelcomeMessage };
