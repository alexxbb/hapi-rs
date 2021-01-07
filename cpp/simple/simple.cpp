#include <HAPI/HAPI.h>
#include <HAPI/HAPI_Common.h>
#include <HAPI/HAPI_Version.h>
#include <HAPI/HAPI_Helpers.h>
#include <iostream>

using namespace std;

#define ENSURE_SUCCESS(result) \
if ( (result) != HAPI_RESULT_SUCCESS ) \
{ \
    std::cout << "Failure at " << __FILE__ << ": " << __LINE__ << std::endl; \
    exit( 1 ); \
}

static std::string
getString(HAPI_StringHandle stringHandle, HAPI_Session &session) {
    if (stringHandle == 0) {
        return "";
    }

    int bufferLength;
    HAPI_GetStringBufLength(&session,
                            stringHandle,
                            &bufferLength);

    char *buffer = new char[bufferLength];

    HAPI_GetString(&session, stringHandle, buffer, bufferLength);

    std::string result(buffer);
    delete[] buffer;

    return result;
}

int
main(int argc, char **argv) {
    HAPI_Session session;
    ENSURE_SUCCESS(HAPI_CreateInProcessSession(&session));

    HAPI_CookOptions cookOptions = HAPI_CookOptions_Create();
    ENSURE_SUCCESS(HAPI_Initialize(&session,
                                   &cookOptions,
                                   false,
                                   -1,
                                   nullptr,
                                   nullptr,
                                   nullptr,
                                   nullptr,
                                   nullptr));

    const char *hdaFile = "sidefx_spaceship.otl";
    int assetLibId;
    ENSURE_SUCCESS(HAPI_LoadAssetLibraryFromFile(&session, hdaFile, true, &assetLibId));

    HAPI_NodeId nodeId;
    ENSURE_SUCCESS(HAPI_CreateNode(&session, -1, "SideFX::Object/spaceship", "Node", true, &nodeId));

    HAPI_ParmInfo info = HAPI_ParmInfo_Create();
    ENSURE_SUCCESS(HAPI_GetParmInfoFromName(
            &session,
            nodeId,
            "stdswitcher3",
            &info));

    std::cout << "Name: " << getString(info.nameSH, session) << std::endl;
    std::cout << "Label: " << getString(info.labelSH, session) << std::endl;

    return 0;
}