exports.getConnectorCandidateURI = function() {

    return new Promise(function(resolve, reject) {
        SDK.Discover.getConnectorCandidate()
            .then(candidate => {
                resolve(`http://${candidate.ip}`)
            })
            .catch(err => {
                reject(err);
            })
    });
}