const explorerGet = require('../../Common/ExplorerHelper').explorerGet;

exports.AuthService =
    {
        getChallenge: (identifier) => {
            return explorerGet(`/auth/challenge/${identifier}`)
        }
    }


