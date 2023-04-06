const jayson = require('jayson/promise');
const oclif = require('@oclif/core');

/**
 *
 * @return {httpApi}
 */
function httpApiFactory() {
  async function httpApi(container) {
    const config = container.resolve('config');

    const server = new jayson.Server({}, {
      router(method, params) {
        const argv = method.split('_');

        // map arguments to argv
        if (Array.isArray(params)) {
          argv.push(...params);
        } else {
          for (const param of Object.keys(params)) {
            argv.push(`--${param}=${params[param]}`);
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

    const port = config.get('dashmate.helper.jsonRpc.port');

    server
      .http()
      // eslint-disable-next-line no-console
      .listen(port, () => console.log(`Dashmate JSON-RPC API started on port: ${port}`));
  }

  return httpApi;
}

module.exports = httpApiFactory;
