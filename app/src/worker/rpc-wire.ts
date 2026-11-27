// Shared RPC rejection wrapper: handlers throw; clients receive the
// structured WiredRpcError instead of Comlink's stripped Error.

import { throwWired } from './errors';

/** Wraps a handler so rejections reach the client as a wired RpcError. */
export const wire =
  <A extends unknown[], R>(f: (...args: A) => Promise<R>) =>
  (...args: A): Promise<R> =>
    f(...args).catch(throwWired);
