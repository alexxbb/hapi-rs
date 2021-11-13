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

static void cookAndPrintNode(HAPI_Session &session, HAPI_CookOptions &co,
                             HAPI_NodeId nodeId, HAPI_PackedPrimInstancingMode mode);

static void printPartInfo(HAPI_Session &session, HAPI_NodeId nodeId,
                          HAPI_PartId partId, std::string indent);

int
main(int argc, char **argv) {
    const char *hdaFile = argc == 2 ? argv[1] : "otls/PackedPrimitive.hda";

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

    HAPI_NodeId nodeId;
    ENSURE_SUCCESS(HAPI_CreateNode(&session, -1, assetName.c_str(), "PackedPrimitive", false, &nodeId));

    cookAndPrintNode(session, cookOptions, nodeId, HAPI_PACKEDPRIM_INSTANCING_MODE_DISABLED);
    cookAndPrintNode(session, cookOptions, nodeId, HAPI_PACKEDPRIM_INSTANCING_MODE_HIERARCHY);
    cookAndPrintNode(session, cookOptions, nodeId, HAPI_PACKEDPRIM_INSTANCING_MODE_FLAT);

    char in;
    std::cout << "Press enter in terminal to exit." << std::endl;
    std::cin >> in;

    HAPI_Cleanup(&session);

    return 0;
}

static void
printPartInfo(HAPI_Session &session, HAPI_NodeId nodeId,
              HAPI_PartId partId, std::string indent) {
    HAPI_PartInfo partInfo;
    ENSURE_SUCCESS(HAPI_GetPartInfo(&session, nodeId, partId, &partInfo));

    if (partInfo.type == HAPI_PARTTYPE_MESH) {
        std::cout << indent << "Part " << partId << ":" << std::endl;
        std::cout << indent << "    Type = Mesh" << std::endl;
        std::cout << indent << "    Point Count = " << partInfo.pointCount << std::endl;
    } else if (partInfo.type == HAPI_PARTTYPE_CURVE) {
        std::cout << indent << "Part " << partId << ":" << std::endl;
        std::cout << indent << "    Type = Curve" << std::endl;
        std::cout << indent << "    Point Count = " << partInfo.pointCount << std::endl;
    } else if (partInfo.type == HAPI_PARTTYPE_INSTANCER) {
        std::cout << indent << "Part " << partId << ":" << std::endl;
        std::cout << indent << "    Type = Instancer" << std::endl;
        std::cout << indent << "    Point Count = " << partInfo.pointCount << std::endl;
        std::cout << indent << "    Instance Count = " << partInfo.instanceCount << std::endl;
        std::cout << indent << "    Instanced Part Count = " << partInfo.instancedPartCount << std::endl;
        // Get the transforms for each instance.

        std::vector<HAPI_Transform> instanceTransforms(partInfo.instanceCount);
        ENSURE_SUCCESS(HAPI_GetInstancerPartTransforms(&session, nodeId, partId,
                                                       HAPI_RSTORDER_DEFAULT, instanceTransforms.data(),
                                                       0, partInfo.instanceCount));

        // Print the instance transforms:
        std::cout << indent << "    Instance Transforms:" << std::endl;

        for (auto instanceTransform : instanceTransforms) {
            float *p = &instanceTransform.position[0];
            std::cout << indent << "        " << p[0] << ", " << p[1] << ", " << p[2] << std::endl;
        }

        // Get the part ids of the parts being instanced.
        std::vector<HAPI_PartId> instancedPartIds(partInfo.instancedPartCount);
        ENSURE_SUCCESS(HAPI_GetInstancedPartIds(&session, nodeId, partId,
                                                instancedPartIds.data(), 0,
                                                partInfo.instancedPartCount));

        // Print the part infos of all the instanced parts.
        std::cout << indent << "    Instanced Parts:" << std::endl;

        for (auto instancedPartId : instancedPartIds)
            printPartInfo(session, nodeId, instancedPartId, "           -> ");
    }
}

static void
cookAndPrintNode(HAPI_Session &session, HAPI_CookOptions &co,
                 HAPI_NodeId nodeId, HAPI_PackedPrimInstancingMode mode) {
    switch (mode) {
        case HAPI_PACKEDPRIM_INSTANCING_MODE_DISABLED:
            std::cout << "Using: HAPI_PACKEDPRIM_INSTANCING_MODE_DISABLED" << std::endl;
            break;
        case HAPI_PACKEDPRIM_INSTANCING_MODE_HIERARCHY:
            std::cout << "Using: HAPI_PACKEDPRIM_INSTANCING_MODE_HIERARCHY" << std::endl;
            break;
        case HAPI_PACKEDPRIM_INSTANCING_MODE_FLAT:
            std::cout << "Using: HAPI_PACKEDPRIM_INSTANCING_MODE_FLAT" << std::endl;
            break;
    }

    co.packedPrimInstancingMode = mode;

    ENSURE_SUCCESS(HAPI_CookNode(&session, nodeId, &co));

    int cookStatus;
    HAPI_Result cookResult;

    do {
        cookResult = HAPI_GetStatus(&session, HAPI_STATUS_COOK_STATE, &cookStatus);
    } while (cookStatus > HAPI_STATE_MAX_READY_STATE && cookResult == HAPI_RESULT_SUCCESS);

    ENSURE_SUCCESS(cookResult);
    ENSURE_COOK_SUCCESS(cookStatus);

    HAPI_NodeInfo nodeInfo;
    ENSURE_SUCCESS(HAPI_GetNodeInfo(&session, nodeId, &nodeInfo));

    int childCount;
    ENSURE_SUCCESS(HAPI_ComposeChildNodeList(&session, nodeId,
                                             HAPI_NODETYPE_SOP, HAPI_NODEFLAGS_ANY,
                                             false, &childCount));

    HAPI_NodeId *childIds = new HAPI_NodeId[childCount];
    ENSURE_SUCCESS(HAPI_GetComposedChildNodeList(&session, nodeId, childIds, childCount));

    for (int i = 0; i < childCount; ++i) {
        HAPI_GeoInfo geoInfo;
        ENSURE_SUCCESS(HAPI_GetGeoInfo(&session, childIds[i], &geoInfo));
        std::cout << "Part count for geo node " << i << ": " << geoInfo.partCount << std::endl;

        for (int j = 0; j < geoInfo.partCount; ++j) {
            printPartInfo(session, childIds[i], j, "");
        }
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
