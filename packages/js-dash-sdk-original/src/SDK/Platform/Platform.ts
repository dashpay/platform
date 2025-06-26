// TODO remove default exports
// eslint-disable-next-line no-restricted-exports
import * as _DashPlatformProtocol from '@dashevo/wasm-dpp';
import { Platform as PlatformClient } from '../Client/Platform/Platform';

export namespace Platform {
  export const DashPlatformProtocol = _DashPlatformProtocol;
  export const { initializeDppModule } = PlatformClient;
}
// eslint-disable-next-line no-restricted-exports
export { Platform as default };
