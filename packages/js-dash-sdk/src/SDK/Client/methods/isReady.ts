/**
 * Report state in a fixed interval of time [100 ms]
 *
 * @param this bound class instance
 *
 * @remarks
 * isReady calls isSelfReady to check report state
 */
const isSelfReady = async function(this: any): Promise<boolean>{
    const self = this;
    return new Promise((res)=>{
        let isReadyInterval = setInterval(() => {
            if (self.state.isReady) {
                clearInterval(isReadyInterval);
                res(true);
            }
        }, 100);
    })
}

/**
 * Check if this instance state is reported ready
 *
 * @param this bound class instance
 * @returns true only if this is ready
 */
async function isReady(this: any) {
    const {state, account} = this;
    if (state.isAccountReady && state.isReady) {
        return true;
    };
    let promises: Promise<boolean>[] = []
    if(!state.isAccountReady && account){
        // @ts-ignore
        promises.push(account.isReady());
    }
    if(!state.isReady){
        promises.push(isSelfReady.call(this));
    }

    await Promise.all(promises);
    return true;
}
export default isReady;
