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

int
main(int argc, char **argv) {
    const char *hdaFile = argc == 2 ? argv[1] : "otls/SideFX_spaceship.otl";

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
    ENSURE_SUCCESS(HAPI_CreateNode(&session, -1, assetName.c_str(), "AnAsset", false, &nodeId));

    ENSURE_SUCCESS(HAPI_CookNode(&session, nodeId, &cookOptions));

    int cookStatus;
    HAPI_Result cookResult;

    do {
        cookResult = HAPI_GetStatus(&session, HAPI_STATUS_COOK_STATE, &cookStatus);
    } while (cookStatus > HAPI_STATE_MAX_READY_STATE && cookResult == HAPI_RESULT_SUCCESS);

    ENSURE_SUCCESS(cookResult);
    ENSURE_COOK_SUCCESS(cookStatus);

    HAPI_NodeInfo nodeInfo;
    ENSURE_SUCCESS(HAPI_GetNodeInfo(&session, nodeId, &nodeInfo));

    HAPI_ParmInfo *parmInfos = new HAPI_ParmInfo[nodeInfo.parmCount];
    ENSURE_SUCCESS(HAPI_GetParameters(&session, nodeId, parmInfos, 0, nodeInfo.parmCount));

    // Print parameter info
    std::cout << "Parameters: " << std::endl;

    for (int i = 0; i < nodeInfo.parmCount; ++i) {
        std::cout << "==========" << std::endl;

        std::cout << "   Name: "
                  << getString(parmInfos[i].nameSH)
                  << std::endl;

        std::cout << "   Values: (";

        if (HAPI_ParmInfo_IsInt(&parmInfos[i])) {
            int parmIntCount = HAPI_ParmInfo_GetIntValueCount(&parmInfos[i]);

            int *parmIntValues = new int[parmIntCount];

            ENSURE_SUCCESS(HAPI_GetParmIntValues(&session,
                                                 nodeId, parmIntValues,
                                                 parmInfos[i].intValuesIndex,
                                                 parmIntCount));

            for (int v = 0; v < parmIntCount; ++v) {
                if (v != 0)
                    std::cout << ", ";

                std::cout << parmIntValues[v];
            }

            delete[] parmIntValues;
        } else if (HAPI_ParmInfo_IsFloat(&parmInfos[i])) {
            int parmFloatCount = HAPI_ParmInfo_GetFloatValueCount(&parmInfos[i]);

            float *parmFloatValues = new float[parmFloatCount];

            ENSURE_SUCCESS(HAPI_GetParmFloatValues(&session,
                                                   nodeId, parmFloatValues,
                                                   parmInfos[i].floatValuesIndex,
                                                   parmFloatCount));

            for (int v = 0; v < parmFloatCount; ++v) {
                if (v != 0)
                    std::cout << ", ";

                std::cout << parmFloatValues[v];
            }

            delete[] parmFloatValues;
        } else if (HAPI_ParmInfo_IsString(&parmInfos[i])) {
            int parmStringCount = HAPI_ParmInfo_GetStringValueCount(&parmInfos[i]);

            HAPI_StringHandle *parmSHValues = new HAPI_StringHandle[parmStringCount];

            ENSURE_SUCCESS(HAPI_GetParmStringValues(&session,
                                                    nodeId,
                                                    true, parmSHValues,
                                                    parmInfos[i].stringValuesIndex,
                                                    parmStringCount));

            for (int v = 0; v < parmStringCount; ++v) {
                if (v != 0)
                    std::cout << ", ";

                std::cout << getString(parmSHValues[v]);
            }

            delete[] parmSHValues;
        }

        std::cout << ")" << std::endl;
    }

    delete[] parmInfos;

    char in;
    std::cout << "Press any key to exit" << std::endl;
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
