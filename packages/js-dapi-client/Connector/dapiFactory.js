DAPISDK = () => {
    return {
        Accounts: require('../Accounts/').Accounts(),
        Explorer: require('../Explorer/').Explorer(),
        Discover: require('../Discover/').Discover(),
        BWS: require('../BWS/').BWS(),
        _config: require('../config.js')
    }
}

module.exports = DAPISDK