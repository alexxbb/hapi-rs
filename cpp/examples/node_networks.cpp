#include <HAPI/HAPI.h>
#include <iostream>
#include <string>
#include <vector>

#define ENSURE_SUCCESS(result) \
if ( (result) != HAPI_RESULT_SUCCESS ) \
{ \
    std::cout << "Failure at " << __FILE__ << ": " << __LINE__ << std::endl; \
    std::cout << getLastError() << std::endl; \
    exit( 1 ); \
}

#define ENSURE_COOK_SUCCESS(result) \
if ( (result) != HAPI_RESULT_SUCCESS ) \
{ \
    std::cout << "Failure at " << __FILE__ << ": " << __LINE__ << std::endl; \
    std::cout << getLastCookError() << std::endl; \
    exit( 1 ); \
}

static std::string getLastError();

static std::string getLastCookError();

static std::string getString(HAPI_StringHandle stringHandle);

static void printChildNodeInfo(HAPI_Session &session, std::vector<HAPI_NodeId> &childrenNodes);

int
main(int argc, char **argv) {
    const char *hdaFile = argc == 2 ? argv[1] : "otls/FourShapes.hda";

    HAPI_CookOptions cookOptions = HAPI_CookOptions_Create();

    HAPI_Session session;

    HAPI_CreateInProcessSession(&session);

    ENSURE_SUCCESS(HAPI_Initialize(&session,
                                   &cookOptions,
                                   true,
                                   -1,
                                   nullptr,
                                   nullptr,
                                   nullptr,
                                   nullptr,
                                   nullptr));

    HAPI_AssetLibraryId assetLibId;
    ENSURE_SUCCESS(HAPI_LoadAssetLibraryFromFile(&session, hdaFile, true, &assetLibId));

    int assetCount;
    ENSURE_SUCCESS(HAPI_GetAvailableAssetCount(&session, assetLibId, &assetCount));

    if (assetCount > 1) {
        std::cout << "Should only be loading 1 asset here" << std::endl;
        exit(1);
    }

    HAPI_StringHandle assetSh;
    ENSURE_SUCCESS(HAPI_GetAvailableAssets(&session, assetLibId, &assetSh, assetCount));

    std::string assetName = getString(assetSh);

    HAPI_NodeId editableNetworkId;
    ENSURE_SUCCESS(HAPI_CreateNode(&session, -1, assetName.c_str(), "FourShapes", false, &editableNetworkId));

    ENSURE_SUCCESS(HAPI_CookNode(&session, editableNetworkId, &cookOptions));

    int cookStatus;
    HAPI_Result cookResult;

    do {
        cookResult = HAPI_GetStatus(&session, HAPI_STATUS_COOK_STATE, &cookStatus);
    } while (cookStatus > HAPI_STATE_MAX_READY_STATE && cookResult == HAPI_RESULT_SUCCESS);

    ENSURE_SUCCESS(cookResult);
    ENSURE_COOK_SUCCESS(cookStatus);

    int childCount;
    ENSURE_SUCCESS(HAPI_ComposeChildNodeList(&session, editableNetworkId,
                                             HAPI_NODETYPE_ANY, HAPI_NODEFLAGS_ANY,
                                             false, &childCount));

    std::cout << "Editable Node Network Child Count: "
              << childCount << std::endl;

    std::vector<HAPI_NodeId> childNodeIds(childCount);
    ENSURE_SUCCESS(HAPI_GetComposedChildNodeList(&session, editableNetworkId,
                                                 &childNodeIds.front(), childCount));

    printChildNodeInfo(session, childNodeIds);

    HAPI_NodeId anotherBoxNode;
    ENSURE_SUCCESS(HAPI_CreateNode(&session, editableNetworkId, "geo",
                                   "ProgrammaticBox", false, &anotherBoxNode));

    ENSURE_SUCCESS(HAPI_ConnectNodeInput(&session, anotherBoxNode, 0, childNodeIds[0], 0));

    ENSURE_SUCCESS(HAPI_CookNode(&session, anotherBoxNode, &cookOptions));

    int boxCookStatus;
    HAPI_Result boxCookResult;

    do {
        boxCookResult = HAPI_GetStatus(&session, HAPI_STATUS_COOK_STATE, &boxCookStatus);
    } while (boxCookStatus > HAPI_STATE_MAX_READY_STATE && boxCookResult == HAPI_RESULT_SUCCESS);

    ENSURE_SUCCESS(boxCookResult);
    ENSURE_COOK_SUCCESS(boxCookStatus);

    // Confirm the connection
    HAPI_NodeId connectedNodeId;
    ENSURE_SUCCESS(HAPI_QueryNodeInput(&session, anotherBoxNode, 0, &connectedNodeId));

    if (connectedNodeId != childNodeIds[0]) {
        std::cout << "The connected node id is " << connectedNodeId << " when it should be "
                  << editableNetworkId << std::endl;
        exit(1);
    }

    ENSURE_SUCCESS(HAPI_ComposeChildNodeList(&session, editableNetworkId,
                                             HAPI_NODETYPE_ANY, HAPI_NODEFLAGS_ANY,
                                             false, &childCount));

    std::vector<HAPI_NodeId> newChildNodes(childCount);

    ENSURE_SUCCESS(HAPI_GetComposedChildNodeList(&session, editableNetworkId,
                                                 &newChildNodes.front(), childCount));

    std::cout << "After CONNECT NODE" << std::endl;
    printChildNodeInfo(session, newChildNodes);

    ENSURE_SUCCESS(HAPI_SaveHIPFile(&session, "otls/modifiedScene.hip", false));

    ENSURE_SUCCESS(HAPI_DisconnectNodeInput(&session, anotherBoxNode, 0));
    ENSURE_SUCCESS(HAPI_DeleteNode(&session, anotherBoxNode));

    std::cout << "After DELETING NODE" << std::endl;

    ENSURE_SUCCESS(HAPI_ComposeChildNodeList(&session, editableNetworkId,
                                             HAPI_NODETYPE_ANY, HAPI_NODEFLAGS_ANY,
                                             false, &childCount));

    std::vector<HAPI_NodeId> finalChildList(childCount);

    ENSURE_SUCCESS(HAPI_GetComposedChildNodeList(&session, editableNetworkId,
                                                 &finalChildList.front(), childCount));


    printChildNodeInfo(session, finalChildList);

    char in;
    std::cout << "Press any key to exit" << std::endl;
    std::cin >> in;

    HAPI_Cleanup(&session);

    return 0;
}

static void printChildNodeInfo(HAPI_Session &session, std::vector<HAPI_NodeId> &childrenNodes) {
    std::cout << "Child Node Ids" << std::endl;
    for (int i = 0; i < childrenNodes.size(); ++i) {
        HAPI_NodeInfo nInfo;
        ENSURE_SUCCESS(HAPI_GetNodeInfo(&session, childrenNodes[i], &nInfo));

        std::cout << "   "
                  << childrenNodes[i]
                  << " - "
                  << (nInfo.createdPostAssetLoad ? "NEW" : "EXISTING")
                  << std::endl;
    }
}

static std::string
getLastError() {
    int bufferLength;
    HAPI_GetStatusStringBufLength(nullptr,
                                  HAPI_STATUS_CALL_RESULT,
                                  HAPI_STATUSVERBOSITY_ERRORS,
                                  &bufferLength);

    char *buffer = new char[bufferLength];

    HAPI_GetStatusString(nullptr, HAPI_STATUS_CALL_RESULT, buffer, bufferLength);

    std::string result(buffer);
    delete[] buffer;

    return result;
}

static std::string
getLastCookError() {
    int bufferLength;
    HAPI_GetStatusStringBufLength(nullptr,
                                  HAPI_STATUS_COOK_RESULT,
                                  HAPI_STATUSVERBOSITY_ERRORS,
                                  &bufferLength);

    char *buffer = new char[bufferLength];

    HAPI_GetStatusString(nullptr, HAPI_STATUS_COOK_RESULT, buffer, bufferLength);

    std::string result(buffer);
    delete[] buffer;

    return result;
}

static std::string
getString(HAPI_StringHandle stringHandle) {
    if (stringHandle == 0) {
        return "";
    }

    int bufferLength;
    HAPI_GetStringBufLength(nullptr,
                            stringHandle,
                            &bufferLength);

    char *buffer = new char[bufferLength];

    HAPI_GetString(nullptr, stringHandle, buffer, bufferLength);

    std::string result(buffer);
    delete[] buffer;

    return result;
}
