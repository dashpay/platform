const jayson = require('jayson/promise')
const oclif = require('@oclif/core')

/**
 *
 * @return {httpApi}
 */
function httpApiFactory() {
  async function httpApi(container) {
    const config = container.resolve('config')

    const server = new jayson.Server({}, {
      router: function (method, params) {
        const argv = method.split('_')

        // map arguments to argv
        if (Array.isArray(params)) {
          argv.push(...params)
        } else {
          for (const param of Object.keys(params)) {
            argv.push(`--${param}=${params[param]}`)
          }
        }

        return new jayson.Method(async () => {
          try {
            return await oclif.run([...argv, `--config=${config.name}`])
          } catch (e) {
            throw server.error(501, e.message);
          }
        });
      }
    });

    server
      .http()
      .listen(9000, () =>
        console.log('Api started on port: 9000')
      );
  }

  return httpApi;
}

module.exports = httpApiFactory;
