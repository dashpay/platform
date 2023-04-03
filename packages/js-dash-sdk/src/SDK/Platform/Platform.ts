// @ts-ignore
import { default as _DashPlatformProtocol } from '@dashevo/dpp';
import { Platform as PlatformClient } from '../Client/Platform/Platform';

export namespace Platform {
  export const DashPlatformProtocol = _DashPlatformProtocol;
  export const { initializeDppModule } = PlatformClient;
}
export { Platform as default };
