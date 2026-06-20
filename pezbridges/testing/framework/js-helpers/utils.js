module.exports = {
    logEvents: function(events) {
        let stringifiedEvents = "";
        events.forEach((record) => {
            if (stringifiedEvents != "") {
                stringifiedEvents += ", ";
            }
            stringifiedEvents += record.event.section + "::" + record.event.method;
        });
        console.log("Block events: " + stringifiedEvents);
    },
    countGrandpaHeaderImports: function(bridgedChain, events) {
        return events.reduce(
            (count, record) => {
                const { event } = record;
                if (event.section == bridgedChain.grandpaPalletName && event.method == "UpdatedBestFinalizedHeader") {
                    count += 1;
                }
                return count;
            },
            0,
        );
    },
    countTeyrchainHeaderImports: function(bridgedChain, events) {
        return events.reduce(
            (count, record) => {
                const { event } = record;
                if (event.section == bridgedChain.teyrchainsPalletName && event.method == "UpdatedTeyrchainHead") {
                    count += 1;
                }
                return count;
            },
            0,
        );
    },
    pollUntil: async function(
        timeoutInSecs,
        predicate,
        cleanup,
        onFailure,
    )  {
        const begin = new Date().getTime();
        const end = begin + timeoutInSecs * 1000;
        while (new Date().getTime() < end) {
            if (predicate()) {
                cleanup();
                return;
            }
            await new Promise(resolve => setTimeout(resolve, 100));
        }

        cleanup();
        onFailure();
    },
    ensureOnlyMandatoryGrandpaHeadersImported: async function(
        bridgedChain,
        apiAtParent,
        apiAtCurrent,
        currentEvents,
    ) {
        // remember id of bridged relay chain GRANDPA authorities set at parent block
        const authoritySetAtParent = await apiAtParent.query[bridgedChain.grandpaPalletName].currentAuthoritySet();
        const authoritySetIdAtParent = authoritySetAtParent["setId"];

        // now read the id of bridged relay chain GRANDPA authorities set at current block
        const authoritySetAtCurrent = await apiAtCurrent.query[bridgedChain.grandpaPalletName].currentAuthoritySet();
        const authoritySetIdAtCurrent = authoritySetAtCurrent["setId"];

        // we expect to see no more than `authoritySetIdAtCurrent - authoritySetIdAtParent` new GRANDPA headers
        const maxNewGrandpaHeaders = authoritySetIdAtCurrent - authoritySetIdAtParent;
        const newGrandpaHeaders = module.exports.countGrandpaHeaderImports(bridgedChain, currentEvents);

        // check that our assumptions are correct
        if (newGrandpaHeaders > maxNewGrandpaHeaders) {
            module.exports.logEvents(currentEvents);
            throw new Error("Unexpected relay chain header import: " + newGrandpaHeaders + " / " + maxNewGrandpaHeaders);
        }

        return newGrandpaHeaders;
    },
    ensureOnlyInitialTeyrchainHeaderImported: async function(
        bridgedChain,
        apiAtParent,
        apiAtCurrent,
        currentEvents,
    ) {
        // remember whether we already know bridged teyrchain header at a parent block
        const bestBridgedTeyrchainHeader = await apiAtParent.query[bridgedChain.teyrchainsPalletName].parasInfo(bridgedChain.bridgedBridgeHubParaId);;
        const hasBestBridgedTeyrchainHeader = bestBridgedTeyrchainHeader.isSome;

        // we expect to see: no more than `1` bridged teyrchain header if there were no teyrchain header before.
        const maxNewTeyrchainHeaders = hasBestBridgedTeyrchainHeader ? 0 : 1;
        const newTeyrchainHeaders = module.exports.countTeyrchainHeaderImports(bridgedChain, currentEvents);

        // check that our assumptions are correct
        if (newTeyrchainHeaders > maxNewTeyrchainHeaders) {
            module.exports.logEvents(currentEvents);
            throw new Error("Unexpected teyrchain header import: " + newTeyrchainHeaders + " / " + maxNewTeyrchainHeaders);
        }

        return hasBestBridgedTeyrchainHeader;
    },
}
