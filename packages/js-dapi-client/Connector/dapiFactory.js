//Todo: apply init process for trustedFactory.js
const qDash = require('quorums-dash')

DAPISDK = (options) => {
    global.SDK = {
        Accounts: require('../Accounts/').Accounts(),
        Explorer: require('../Explorer/').Explorer(),
        Discover: require('../Discover/').Discover(),
        BWS: require('../BWS/').BWS(),
        Quorum: require('../Quorums'),
        Blockchain: require('../Blockchain/').Blockchain(),
        SPV: require('../SPV/'),
        _config: require('../config.js'),
    }

    return initDapi();
}

var initDapi = function(useQuorums) {
    return new Promise(function(resolve, reject) {

        SDK.Discover.Masternode.masternodeList = {
            hash: null,
            nodes: SDK._config.DISCOVER.DAPI_SEEDS
        }

        SDK.Discover.Masternode.fetcher()
            .then(fetched => {
                SDK.Discover.Masternode.masternodeList = {
                    hash: qDash.getHash(fetched.list),
                    nodes: fetched.list
                }
                SDK.Discover.Masternode.candidateList = SDK.Discover.Masternode.masternodeList.nodes
                return updateMnList()

            })
            .then(listUpdated => {
                return SDK.Quorum.updateQuorum()
            })
            .then(quorumUpdated => {
                startMnListUpdater()
                resolve(true)
            })
    })
}

//todo: no need to update quorum each time
//only mnList periodically, quorums only on quorum request
var startMnListUpdater = function() {
    setInterval(function() {
        updateMnList()
            .then(res => SDK.Quorum.updateQuorum())
    }, 60 * 1 * 1000) //1min todo: move to config
}

var updateMnList = function() {
    return SDK.Discover.Masternode.fetcher()
        .then(fetched => {
            switch (fetched.type) {
                case 'full':
                    SDK.Discover.Masternode.masternodeList = {
                        hash: qDash.getHash(fetched.list),
                        nodes: fetched.list
                    }
                    break;
                case 'update':
                    //todo: improve code
                    SDK.Discover.Masternode.masternodeList.nodes =
                        SDK.Discover.Masternode.masternodeList.nodes.filter(n => n.vin != fetched.list.deletions)
                    SDK.Discover.Masternode.masternodeList.nodes = SDK.Discover.Masternode.masternodeList.nodes.concat(fetched.list.additions)
                    SDK.Discover.Masternode.masternodeList.hash = qDash.getHash(SDK.Discover.Masternode.masternodeList.nodes)
                    break;
                case 'none':
                    //Nothing to do    
                    break;
            }
        })
}

module.exports = DAPISDK