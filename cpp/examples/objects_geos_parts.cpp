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

static std::string getString(HAPI_StringHandle stringHandle);

static void printCompleteNodeInfo(HAPI_Session &session, HAPI_NodeId nodeId,
                                  HAPI_AssetInfo &assetInfo);

static void processGeoPart(HAPI_Session &session, HAPI_AssetInfo &assetInfo,
                           HAPI_NodeId objectNode, HAPI_NodeId geoNode,
                           HAPI_PartId partId);

static void processFloatAttrib(HAPI_Session &session, HAPI_AssetInfo &assetInfo,
                               HAPI_NodeId objectNode, HAPI_NodeId geoNode,
                               HAPI_PartId partId, HAPI_AttributeOwner owner,
                               std::string name);

int
main(int argc, char **argv) {
    const char *hdaFile = argc == 2 ? argv[1] : "otls/TestShapes.hda";

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
    ENSURE_SUCCESS(HAPI_CreateNode(&session, -1, assetName.c_str(), "TestObject", false, &nodeId));

    ENSURE_SUCCESS(HAPI_CookNode(&session, nodeId, &cookOptions));

    int cookStatus;
    HAPI_Result cookResult;

    do {
        cookResult = HAPI_GetStatus(&session, HAPI_STATUS_COOK_STATE, &cookStatus);
    } while (cookStatus > HAPI_STATE_MAX_READY_STATE && cookResult == HAPI_RESULT_SUCCESS);

    ENSURE_SUCCESS(cookResult);
    ENSURE_COOK_SUCCESS(cookStatus);

    HAPI_AssetInfo assetInfo;
    ENSURE_SUCCESS(HAPI_GetAssetInfo(&session, nodeId, &assetInfo));

    printCompleteNodeInfo(session, nodeId, assetInfo);

    char in;
    std::cout << "Press keys to exit." << std::endl;
    std::cin >> in;

    HAPI_Cleanup(&session);

    return 0;
}

static void
printCompleteNodeInfo(HAPI_Session &session, HAPI_NodeId nodeId,
                      HAPI_AssetInfo &assetInfo) {
    int objectCount;
    ENSURE_SUCCESS(HAPI_ComposeObjectList(&session, nodeId,
                                          nullptr, &objectCount));

    HAPI_ObjectInfo *objectInfos =
            new HAPI_ObjectInfo[objectCount];

    ENSURE_SUCCESS(HAPI_GetComposedObjectList(&session, nodeId,
                                              objectInfos, 0, objectCount));


    for (int objectIndex = 0; objectIndex < objectCount; ++objectIndex) {
        HAPI_ObjectInfo &objectInfo = objectInfos[objectIndex];

        HAPI_GeoInfo geoInfo;
        ENSURE_SUCCESS(HAPI_GetDisplayGeoInfo(&session, objectInfo.nodeId, &geoInfo));

        for (int partIndex = 0; partIndex < geoInfo.partCount; ++partIndex) {
            processGeoPart(session, assetInfo, objectInfo.nodeId,
                           geoInfo.nodeId, partIndex);
        }
    }
}

static void
processFloatAttrib(HAPI_Session &session, HAPI_AssetInfo &assetInfo,
                   HAPI_NodeId objectNode, HAPI_NodeId geoNode,
                   HAPI_PartId partId, HAPI_AttributeOwner owner,
                   std::string name) {
    HAPI_AttributeInfo attribInfo;
    ENSURE_SUCCESS(HAPI_GetAttributeInfo(&session, geoNode, partId,
                                         name.c_str(), owner, &attribInfo));


    float *attribData = new float[attribInfo.count * attribInfo.tupleSize];

    ENSURE_SUCCESS(HAPI_GetAttributeFloatData(&session, geoNode, partId,
                                              name.c_str(), &attribInfo, -1,
                                              attribData, 0, attribInfo.count));

    for (int elemIndex = 0; elemIndex < attribInfo.count; ++elemIndex) {
        for (int tupleIndex = 0; tupleIndex < attribInfo.tupleSize;
             ++tupleIndex) {
            std::cout << attribData[
                    elemIndex * attribInfo.tupleSize + tupleIndex]
                      << " ";
        }
        std::cout << std::endl;
    }
    delete[] attribData;
}

static void
processGeoPart(HAPI_Session &session, HAPI_AssetInfo &assetInfo,
               HAPI_NodeId objectNode, HAPI_NodeId geoNode,
               HAPI_PartId partId) {
    std::cout << "Object " << objectNode << ", Geo " << geoNode
              << ", Part " << partId << std::endl;

    HAPI_PartInfo partInfo;
    ENSURE_SUCCESS(HAPI_GetPartInfo(&session, geoNode,
                                    partId, &partInfo));

    HAPI_StringHandle *attribNamesSh = new HAPI_StringHandle[partInfo.attributeCounts[HAPI_ATTROWNER_POINT]];

    ENSURE_SUCCESS(HAPI_GetAttributeNames(&session, geoNode, partInfo.id,
                                          HAPI_ATTROWNER_POINT, attribNamesSh,
                                          partInfo.attributeCounts[HAPI_ATTROWNER_POINT]));

    for (int attribIndex = 0; attribIndex < partInfo.attributeCounts[HAPI_ATTROWNER_POINT]; ++attribIndex) {
        std::string attribName = getString(attribNamesSh[attribIndex]);
        std::cout << "      " << attribName << std::endl;
    }

    delete[] attribNamesSh;

    std::cout << "Point Positions: " << std::endl;

    processFloatAttrib(session, assetInfo, objectNode, geoNode,
                       partId, HAPI_ATTROWNER_POINT, "P");

    std::cout << "Number of Faces: " << partInfo.faceCount << std::endl;

    if (partInfo.type != HAPI_PARTTYPE_CURVE) {
        int *faceCounts = new int[partInfo.faceCount];

        ENSURE_SUCCESS(HAPI_GetFaceCounts(&session, geoNode, partId,
                                          faceCounts, 0, partInfo.faceCount));

        for (int ii = 0; ii < partInfo.faceCount; ++ii) {
            std::cout << faceCounts[ii] << ", ";
        }

        std::cout << std::endl;

        int *vertexList = new int[partInfo.vertexCount];
        ENSURE_SUCCESS(HAPI_GetVertexList(&session, geoNode, partId,
                                          vertexList, 0, partInfo.vertexCount));

        std::cout << "Vertex Indices into Points array:" << std::endl;
        int currIndex = 0;
        for (int ii = 0; ii < partInfo.faceCount; ii++) {
            for (int jj = 0; jj < faceCounts[ii]; jj++) {
                std::cout
                        << "Vertex :" << currIndex << ", belonging to face: "
                        << ii << ", index: "
                        << vertexList[currIndex] << " of points array\n";
                currIndex++;
            }
        }

        delete[] faceCounts;
        delete[] vertexList;
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
