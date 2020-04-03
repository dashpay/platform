const browserify = require('browserify');
const through = require('through2');

/**
 * @return {Promise<string>}
 */
async function compileJsonSchemaValidatorCode() {
  return new Promise((resolve, reject) => {
    browserify(
      require.resolve('./internal/initializeValidator'),
      { standalone: 'jsonSchemaValidator' },
    ).transform(() => (
      through(function transformUriJS(buf, enc, next) {
        this.push(buf.toString('utf8')
          .replace(
            /require\('uri-js'\)/g,
            `require('${require.resolve('./internal/UriJSShim')}')`,
          ));
        next();
      })
    ), {
      global: true,
    }).bundle((err, buf) => {
      if (err) {
        return reject(err);
      }
      return resolve(buf.toString());
    });
  });
}

module.exports = compileJsonSchemaValidatorCode;
