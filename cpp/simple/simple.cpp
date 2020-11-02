#include <HAPI/HAPI.h>
#include <HAPI/HAPI_Common.h>
#include <HAPI/HAPI_Version.h>
#include <HAPI/HAPI_Helpers.h>
#include <iostream>
using namespace std;

int
main( int argc, char **argv )
{
    cout<<"main starts"<<endl;
    HAPI_CookOptions cookOptions = HAPI_CookOptions_Create();
    cout<<"options created"<<endl;
    HAPI_Session session;
    cout<<"desired Result: "<<HAPI_RESULT_SUCCESS<<endl;
    HAPI_Result SessionResult = HAPI_CreateInProcessSession( &session );
    cout<<"SessionResult: "<<SessionResult<<endl;

    HAPI_Result initResult = HAPI_Initialize( &session,
				     &cookOptions,
				     true,
				     -1,
				     nullptr,
				     nullptr,
				     nullptr,
				     nullptr,
				     nullptr );
    cout<<"initResult: "<<initResult<<endl;

    HAPI_Result cleanupResult = HAPI_Cleanup( &session );
    cout<<"cleanupResult: "<<cleanupResult<<endl;

    return 0;
}