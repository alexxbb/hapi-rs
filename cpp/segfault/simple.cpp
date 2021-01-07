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

    int parm_count;
    int c1;
    int c2;
    int c3;
    int c4;
    ENSURE_SUCCESS(HAPI_GetAssetDefinitionParmCounts(&session,
                                                     assetLibId,
                                                     "SideFX::Object/spaceship",
                                                     &parm_count,
                                                     &c1,
                                                     &c2,
                                                     &c3,
                                                     &c4));

    return 0;
}