import jayson from 'jayson/promise/index.js';

export default function createHttpApiServerFactory() {
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
            const { execute } = await import('@oclif/core');
            return await execute({ dir: import.meta.url, args: argv });
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
