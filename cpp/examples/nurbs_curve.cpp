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
    HAPI_NodeId curveNode;

    ENSURE_SUCCESS(HAPI_CreateNode(&session, -1, "sop/curve", "NURBS", false, &curveNode));

    ENSURE_SUCCESS(HAPI_CookNode(&session, curveNode, &cookOptions));

    int cookStatus;
    HAPI_Result cookResult;

    do {
        cookResult = HAPI_GetStatus(&session, HAPI_STATUS_COOK_STATE, &cookStatus);
    } while (cookStatus > HAPI_STATE_MAX_READY_STATE && cookResult == HAPI_RESULT_SUCCESS);

    ENSURE_SUCCESS(cookResult);
    ENSURE_COOK_SUCCESS(cookStatus);

    HAPI_NodeInfo curveNodeInfo;
    ENSURE_SUCCESS(HAPI_GetNodeInfo(&session, curveNode, &curveNodeInfo));

    HAPI_ParmInfo *parmInfos = new HAPI_ParmInfo[curveNodeInfo.parmCount];
    ENSURE_SUCCESS(HAPI_GetParameters(&session, curveNode, parmInfos, 0, curveNodeInfo.parmCount));

    int coordsParmIndex = -1;
    int typeParmIndex = -1;

    for (int i = 0; i < curveNodeInfo.parmCount; i++) {
        std::string parmName = getString(parmInfos[i].nameSH);

        if (parmName == "coords") {
            coordsParmIndex = i;
        }
        if (parmName == "type") {
            typeParmIndex = i;
        }
    }

    if (coordsParmIndex == -1 || typeParmIndex == -1) {
        std::cout << "Failure at " << __FILE__ << ": " << __LINE__ << std::endl;
        std::cout << "Could not find coords/type parameter on curve node" << std::endl;
    }

    HAPI_ParmInfo parm;

    ENSURE_SUCCESS(HAPI_GetParameters(&session, curveNode, &parm, typeParmIndex, 1));

    int typeValue = 1;
    ENSURE_SUCCESS(HAPI_SetParmIntValues(&session, curveNode, &typeValue, parm.intValuesIndex, 1));

    ENSURE_SUCCESS(HAPI_GetParameters(&session, curveNode, &parm, coordsParmIndex, 1));
    ENSURE_SUCCESS(HAPI_SetParmStringValue(&session, curveNode, "-4,0,4 -4,0,-4 4,0,-4 4,0,4", parm.id, 0));

    HAPI_SaveHIPFile(&session, "otls/nurbs_curve.hip", true);

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
