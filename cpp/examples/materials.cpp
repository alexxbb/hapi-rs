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

int
main(int argc, char **argv) {
    const char *hdaFile = argc == 2 ? argv[1] : "../otls/sesi/SideFX_spaceship.otl";

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
    ENSURE_SUCCESS(HAPI_CreateNode(&session, -1, assetName.c_str(), "BrandonTest", false, &nodeId));

    ENSURE_SUCCESS(HAPI_CookNode(&session, nodeId, &cookOptions));

    int cookStatus;
    HAPI_Result cookResult;

    do {
        cookResult = HAPI_GetStatus(&session, HAPI_STATUS_COOK_STATE, &cookStatus);
    } while (cookStatus > HAPI_STATE_MAX_READY_STATE && cookResult == HAPI_RESULT_SUCCESS);

    ENSURE_SUCCESS(cookResult);
    ENSURE_COOK_SUCCESS(cookStatus);

    HAPI_GeoInfo geoInfo;
    ENSURE_SUCCESS(HAPI_GetDisplayGeoInfo(&session, nodeId, &geoInfo));

    HAPI_PartInfo partInfo;
    ENSURE_SUCCESS(HAPI_GetPartInfo(&session, geoInfo.nodeId, 0, &partInfo));

    bool areAllTheSame = false;
    std::vector<HAPI_NodeId> materialIds(partInfo.faceCount);
    ENSURE_SUCCESS(HAPI_GetMaterialNodeIdsOnFaces(&session, geoInfo.nodeId, partInfo.id, &areAllTheSame,
                                                  &materialIds.front(), 0, partInfo.faceCount));

    if (!areAllTheSame) {
        std::cout << "All materials should be the same." << std::endl;
        exit(1);
    }

    for (int i = 0; i < partInfo.faceCount; ++i) {
        if (materialIds[i] != materialIds[0]) {
            std::cout << "All material ids should be the same." << std::endl;
            exit(1);
        }
    }

    // The materials are all the same, so we will just extract the first material

    HAPI_MaterialInfo materialInfo;
    ENSURE_SUCCESS(HAPI_GetMaterialInfo(&session, materialIds[0], &materialInfo));

    if (materialInfo.nodeId != materialIds[0] ||
        materialInfo.nodeId < 0 ||
        materialInfo.exists != true ||
        materialInfo.hasChanged != true) {
        std::cout << "Did not successfully extract the first material" << std::endl;
        exit(1);
    }

    HAPI_NodeInfo materialNodeInfo;
    ENSURE_SUCCESS(HAPI_GetNodeInfo(&session, materialInfo.nodeId, &materialNodeInfo));

    std::cout << getString(materialNodeInfo.nameSH) << std::endl;

    HAPI_ParmInfo *parmInfos = new HAPI_ParmInfo[materialNodeInfo.parmCount];
    ENSURE_SUCCESS(HAPI_GetParameters(&session, materialNodeInfo.id, parmInfos,
                                      0, materialNodeInfo.parmCount));

    int baseColorMapIndex = -1;
    for (int i = 0; i < materialNodeInfo.parmCount; ++i) {
        if (getString(parmInfos[i].nameSH) == "baseColorMap") {
            baseColorMapIndex = i;
        }
    }

    if (baseColorMapIndex < 0) {
        std::cout << "Could not find the base color map parameter" << std::endl;
        exit(1);
    }

    HAPI_StringHandle basePath;
    ENSURE_SUCCESS(HAPI_GetParmStringValue(&session, materialNodeInfo.id, "baseColorMap",
                                           0, true, &basePath));

    std::cout << "Base Color Map Path: " << getString(basePath) << std::endl;

    ENSURE_SUCCESS(HAPI_RenderTextureToImage(&session, materialNodeInfo.id, baseColorMapIndex));

    HAPI_ImageInfo imgInfo;
    ENSURE_SUCCESS(HAPI_GetImageInfo(&session, materialNodeInfo.id, &imgInfo));

    std::cout << "Image Width = " << imgInfo.xRes << std::endl
              << "Image Height = " << imgInfo.yRes << std::endl
              << "Image Format = " << getString(imgInfo.imageFileFormatNameSH) << std::endl;

    ENSURE_SUCCESS(HAPI_SetImageInfo(&session, materialNodeInfo.id, &imgInfo));

    int imagePlaneCount;
    ENSURE_SUCCESS(HAPI_GetImagePlaneCount(&session, materialNodeInfo.id, &imagePlaneCount));

    HAPI_StringHandle *imagePlanes = new HAPI_StringHandle[imagePlaneCount];
    ENSURE_SUCCESS(HAPI_GetImagePlanes(&session, materialNodeInfo.id, imagePlanes, imagePlaneCount));

    for (int j = 0; j < imagePlaneCount; ++j) {
        std::string imagePlaneName = getString(imagePlanes[j]);
        std::cout << "Image Plane [ " << j << " ] = " << imagePlaneName << std::endl;

        int destinationFilePath;

        ENSURE_SUCCESS(HAPI_ExtractImageToFile(&session, materialNodeInfo.id, nullptr,
                                               imagePlaneName.c_str(), ".", nullptr, &destinationFilePath));
    }

    char in;
    std::cout << "Enter some input to exit" << std::endl;
    std::cin >> in;

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
