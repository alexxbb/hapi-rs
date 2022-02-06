#include <HAPI/HAPI.h>
#include <iostream>
#include <string>

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

int
main(int argc, char **argv) {
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


    HAPI_NodeId newNode;

    ENSURE_SUCCESS(HAPI_CreateInputNode(&session, &newNode, "Point Cloud"));
    ENSURE_SUCCESS(HAPI_CookNode(&session, newNode, &cookOptions));

    int cookStatus;
    HAPI_Result cookResult;

    do {
        cookResult = HAPI_GetStatus(&session, HAPI_STATUS_COOK_STATE, &cookStatus);
    } while (cookStatus > HAPI_STATE_MAX_READY_STATE && cookResult == HAPI_RESULT_SUCCESS);

    ENSURE_SUCCESS(cookResult);
    ENSURE_COOK_SUCCESS(cookStatus);

    HAPI_GeoInfo newNodeGeoInfo;

    ENSURE_SUCCESS(HAPI_GetDisplayGeoInfo(&session, newNode, &newNodeGeoInfo));

    HAPI_NodeId sopNodeId = newNodeGeoInfo.nodeId;

    // Creating the triangle vertices
    HAPI_PartInfo newNodePart = HAPI_PartInfo_Create();

    newNodePart.type = HAPI_PARTTYPE_MESH;
    newNodePart.faceCount = 0;
    newNodePart.vertexCount = 0;
    newNodePart.pointCount = 8;

    ENSURE_SUCCESS(HAPI_SetPartInfo(&session, sopNodeId, 0, &newNodePart));

    HAPI_AttributeInfo newNodePointInfo = HAPI_AttributeInfo_Create();
    newNodePointInfo.count = 8;
    newNodePointInfo.tupleSize = 3;
    newNodePointInfo.exists = true;
    newNodePointInfo.storage = HAPI_STORAGETYPE_FLOAT;
    newNodePointInfo.owner = HAPI_ATTROWNER_POINT;

    ENSURE_SUCCESS(HAPI_AddAttribute(&session, sopNodeId, 0, "P", &newNodePointInfo));

    float positions[24] = {0.0f, 0.0f, 0.0f,
                           1.0f, 0.0f, 0.0f,
                           1.0f, 0.0f, 1.0f,
                           0.0f, 0.0f, 1.0f,
                           0.0f, 1.0f, 0.0f,
                           1.0f, 1.0f, 0.0f,
                           1.0f, 1.0f, 1.0f,
                           0.0f, 1.0f, 1.0f};

    ENSURE_SUCCESS(HAPI_SetAttributeFloatData(&session, sopNodeId, 0, "P", &newNodePointInfo, positions, 0, 8));

    ENSURE_SUCCESS(HAPI_CommitGeo(&session, sopNodeId));

    ENSURE_SUCCESS(HAPI_SaveHIPFile(&session, "otls/geometry_point_cloud.hip", false));

    HAPI_Cleanup(&session);

    return 0;
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
