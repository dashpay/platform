const RetriableResponseError = require('./RetriableResponseError');

class ServerError extends RetriableResponseError {

}

module.exports = ServerError;
