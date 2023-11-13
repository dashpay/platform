import jayson from 'jayson/promise'
import oclif from '@oclif/core';

export function createHttpApiServerFactory() {
  /**
   * @return {HttpServer}
   */
  function createHttpApiServer() {
    const server = new jayson.Server({}, {
      router(method, params) {
        const argv = method.split(' ');

        // map arguments to argv
        if (Array.isArray(params)) {
          argv.push(...params);
        } else {
          for (const [name, value] of Object.entries(params)) {
            argv.push(`--${name}=${value}`);
          }
        }

        return new jayson.Method(async () => {
          try {
            return await oclif.run([...argv]);
          } catch (e) {
            throw server.error(501, e.message);
          }
        });
      },
    });

    return server.http();
  }

  return createHttpApiServer;
}
