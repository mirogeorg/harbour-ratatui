#include "hbapi.h"

#if ! defined( HB_OS_WIN )
#  error "The current Harbour/Ratatui loader supports Windows only"
#endif

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#if defined( _MSC_VER )
#  define HRUI_WINAPI __stdcall
#  define HRUI_DLLIMPORT __declspec( dllimport )
#else
#  define HRUI_WINAPI __attribute__(( stdcall ))
#  define HRUI_DLLIMPORT __attribute__(( dllimport ))
#endif

/* Keep windows.h out of the bridge.  Some older MinGW64 distributions pull
 * incompatible AVX512 headers from it.  These are the only Win32 declarations
 * required by the dynamic loader and ANSI-console helper. */
typedef void * HRUI_HANDLE;
typedef void * HRUI_HMODULE;
typedef unsigned long HRUI_DWORD;
typedef int HRUI_BOOL;
typedef struct
{
   short x;
   short y;
} HRUI_COORD;

HRUI_DLLIMPORT HRUI_HMODULE HRUI_WINAPI LoadLibraryA( const char * );
HRUI_DLLIMPORT HRUI_BOOL HRUI_WINAPI FreeLibrary( HRUI_HMODULE );
HRUI_DLLIMPORT void * HRUI_WINAPI GetProcAddress( HRUI_HMODULE, const char * );
HRUI_DLLIMPORT HRUI_DWORD HRUI_WINAPI GetLastError( void );
HRUI_DLLIMPORT HRUI_DWORD HRUI_WINAPI GetEnvironmentVariableA( const char *, char *, HRUI_DWORD );
HRUI_DLLIMPORT HRUI_HANDLE HRUI_WINAPI GetStdHandle( HRUI_DWORD );
HRUI_DLLIMPORT HRUI_BOOL HRUI_WINAPI GetConsoleMode( HRUI_HANDLE, HRUI_DWORD * );
HRUI_DLLIMPORT HRUI_BOOL HRUI_WINAPI SetConsoleMode( HRUI_HANDLE, HRUI_DWORD );
HRUI_DLLIMPORT int HRUI_WINAPI MultiByteToWideChar(
   unsigned int, HRUI_DWORD, const char *, int, uint16_t *, int );
HRUI_DLLIMPORT HRUI_BOOL HRUI_WINAPI WriteConsoleW(
   HRUI_HANDLE, const void *, HRUI_DWORD, HRUI_DWORD *, void * );
HRUI_DLLIMPORT HRUI_BOOL HRUI_WINAPI SetConsoleCursorPosition(
   HRUI_HANDLE, HRUI_COORD );

#define HRUI_MAX_PATH 260
#define HRUI_STD_OUTPUT_HANDLE ( ( HRUI_DWORD ) -11 )
#define HRUI_INVALID_HANDLE_VALUE ( ( HRUI_HANDLE ) ( intptr_t ) -1 )
#define HRUI_ENABLE_VIRTUAL_TERMINAL_PROCESSING 0x0004
#define HRUI_CP_UTF8 65001
#define HRUI_MB_ERR_INVALID_CHARS 0x00000008

#define HRUI_ABI_EXPECTED 1u
#define HRUI_OK 0
#define HRUI_BUFFER_TOO_SMALL (-2)

typedef uint32_t ( * HRUI_ABI_VERSION_FN )( void );
typedef int32_t ( * HRUI_RENDER_FN )(
   const uint8_t *, size_t,
   const uint8_t *, size_t,
   uint16_t, uint16_t, uint8_t,
   uint8_t *, size_t, size_t * );
typedef int32_t ( * HRUI_SHOWCASE_FN )(
   uint32_t, size_t, uint16_t, uint16_t, uint8_t,
   uint8_t *, size_t, size_t * );
typedef int32_t ( * HRUI_SHOWCASE_V2_FN )(
   uint32_t, size_t, size_t, size_t, uint32_t, uint8_t,
   uint16_t, uint16_t, uint8_t, uint8_t *, size_t, size_t * );
typedef int32_t ( * HRUI_SHOWCASE_V3_FN )(
   uint32_t, size_t, size_t, size_t, size_t, size_t, uint32_t, uint8_t,
   uint16_t, uint16_t, uint8_t, uint8_t *, size_t, size_t * );
typedef int32_t ( * HRUI_SHOWCASE_V4_FN )(
   uint32_t, size_t, size_t, size_t, size_t, size_t, uint32_t, uint32_t,
   uint8_t, uint16_t, uint16_t, uint8_t, uint8_t *, size_t, size_t * );
typedef int32_t ( * HRUI_RENDER_COMMANDS_FN )(
   const uint8_t *, size_t, uint8_t, uint8_t *, size_t, size_t * );
typedef size_t ( * HRUI_LAST_ERROR_FN )( char *, size_t );

static HRUI_HMODULE s_library = NULL;
static HRUI_ABI_VERSION_FN s_abi_version = NULL;
static HRUI_RENDER_FN s_render = NULL;
static HRUI_SHOWCASE_FN s_showcase = NULL;
static HRUI_SHOWCASE_V2_FN s_showcase_v2 = NULL;
static HRUI_SHOWCASE_V3_FN s_showcase_v3 = NULL;
static HRUI_SHOWCASE_V4_FN s_showcase_v4 = NULL;
static HRUI_RENDER_COMMANDS_FN s_render_commands = NULL;
static HRUI_LAST_ERROR_FN s_last_error = NULL;
static char s_loader_error[ 512 ] = "Ratatui DLL has not been loaded";

static void hrui_set_windows_error( const char * operation )
{
   _snprintf( s_loader_error, sizeof( s_loader_error ) - 1,
              "%s (Windows error %lu)", operation,
              ( unsigned long ) GetLastError() );
   s_loader_error[ sizeof( s_loader_error ) - 1 ] = '\0';
}

static HB_BOOL hrui_load( void )
{
   char dll_path[ HRUI_MAX_PATH ];
   HRUI_DWORD path_length;

   if( s_library != NULL )
      return HB_TRUE;

   path_length = GetEnvironmentVariableA( "HB_RATATUI_DLL", dll_path,
                                          ( HRUI_DWORD ) sizeof( dll_path ) );
   if( path_length == 0 || path_length >= sizeof( dll_path ) )
      strcpy( dll_path, "harbour_ratatui.dll" );

   s_library = LoadLibraryA( dll_path );
   if( s_library == NULL )
   {
      hrui_set_windows_error( "Cannot load harbour_ratatui.dll" );
      return HB_FALSE;
   }

   s_abi_version = ( HRUI_ABI_VERSION_FN ) ( void * )
      GetProcAddress( s_library, "hrui_abi_version" );
   s_render = ( HRUI_RENDER_FN ) ( void * )
      GetProcAddress( s_library, "hrui_render_dashboard" );
   s_showcase = ( HRUI_SHOWCASE_FN ) ( void * )
      GetProcAddress( s_library, "hrui_render_showcase" );
   s_showcase_v2 = ( HRUI_SHOWCASE_V2_FN ) ( void * )
      GetProcAddress( s_library, "hrui_render_showcase_v2" );
   s_showcase_v3 = ( HRUI_SHOWCASE_V3_FN ) ( void * )
      GetProcAddress( s_library, "hrui_render_showcase_v3" );
   s_showcase_v4 = ( HRUI_SHOWCASE_V4_FN ) ( void * )
      GetProcAddress( s_library, "hrui_render_showcase_v4" );
   s_render_commands = ( HRUI_RENDER_COMMANDS_FN ) ( void * )
      GetProcAddress( s_library, "hrui_render_commands" );
   s_last_error = ( HRUI_LAST_ERROR_FN ) ( void * )
      GetProcAddress( s_library, "hrui_last_error" );

   if( s_abi_version == NULL || s_render == NULL || s_last_error == NULL )
   {
      hrui_set_windows_error( "The Ratatui DLL does not export the expected ABI" );
      FreeLibrary( s_library );
      s_library = NULL;
      s_abi_version = NULL;
      s_render = NULL;
      s_showcase = NULL;
      s_showcase_v2 = NULL;
      s_showcase_v3 = NULL;
      s_showcase_v4 = NULL;
      s_render_commands = NULL;
      s_last_error = NULL;
      return HB_FALSE;
   }

   if( s_abi_version() != HRUI_ABI_EXPECTED )
   {
      _snprintf( s_loader_error, sizeof( s_loader_error ) - 1,
                 "Ratatui ABI mismatch: DLL=%lu, Harbour bridge=%lu",
                 ( unsigned long ) s_abi_version(),
                 ( unsigned long ) HRUI_ABI_EXPECTED );
      FreeLibrary( s_library );
      s_library = NULL;
      s_abi_version = NULL;
      s_render = NULL;
      s_showcase = NULL;
      s_showcase_v2 = NULL;
      s_showcase_v3 = NULL;
      s_showcase_v4 = NULL;
      s_render_commands = NULL;
      s_last_error = NULL;
      return HB_FALSE;
   }

   s_loader_error[ 0 ] = '\0';
   return HB_TRUE;
}

HB_FUNC( RTUI_AVAILABLE )
{
   hb_retl( hrui_load() );
}

HB_FUNC( RTUI_ABI_VERSION )
{
   if( hrui_load() )
      hb_retnint( ( HB_MAXINT ) s_abi_version() );
   else
      hb_retni( 0 );
}

HB_FUNC( RTUI_LAST_ERROR )
{
   if( s_loader_error[ 0 ] != '\0' )
      hb_retc( s_loader_error );
   else if( s_library != NULL && s_last_error != NULL )
   {
      size_t required = s_last_error( NULL, 0 );
      char * message = ( char * ) hb_xgrab( ( HB_SIZE ) required + 1 );
      s_last_error( message, required + 1 );
      hb_retclen_buffer( message, ( HB_SIZE ) required );
   }
   else
      hb_retc( s_loader_error );
}

HB_FUNC( RTUI_RENDER )
{
   const char * title;
   const char * body;
   HB_SIZE title_length;
   HB_SIZE body_length;
   int width;
   int height;
   HB_BOOL ansi;
   size_t required = 0;
   size_t written = 0;
   int32_t status;
   char * output;

   if( ! hrui_load() )
   {
      hb_retc_null();
      return;
   }

   title = hb_parc( 1 );
   body = hb_parc( 2 );
   title_length = hb_parclen( 1 );
   body_length = hb_parclen( 2 );
   width = hb_parnidef( 3, 72 );
   height = hb_parnidef( 4, 15 );
   ansi = hb_parldef( 5, HB_TRUE );

   if( title == NULL )
   {
      title = "";
      title_length = 0;
   }
   if( body == NULL )
   {
      body = "";
      body_length = 0;
   }

   status = s_render( ( const uint8_t * ) title, ( size_t ) title_length,
                      ( const uint8_t * ) body, ( size_t ) body_length,
                      ( uint16_t ) width, ( uint16_t ) height,
                      ansi ? 1 : 0, NULL, 0, &required );
   if( status != HRUI_BUFFER_TOO_SMALL || required == 0 )
   {
      hb_retc_null();
      return;
   }

   output = ( char * ) hb_xgrab( ( HB_SIZE ) required + 1 );
   status = s_render( ( const uint8_t * ) title, ( size_t ) title_length,
                      ( const uint8_t * ) body, ( size_t ) body_length,
                      ( uint16_t ) width, ( uint16_t ) height,
                      ansi ? 1 : 0, ( uint8_t * ) output, required, &written );
   if( status != HRUI_OK )
   {
      hb_xfree( output );
      hb_retc_null();
      return;
   }

   output[ written ] = '\0';
   hb_retclen_buffer( output, ( HB_SIZE ) written );
}

HB_FUNC( RTUI_RENDER_COMMANDS )
{
   const char * commands = hb_parc( 1 );
   HB_SIZE commands_length = hb_parclen( 1 );
   HB_BOOL ansi = hb_parldef( 2, HB_TRUE );
   size_t required = 0;
   size_t written = 0;
   int32_t status;
   char * output;

   if( commands == NULL )
   {
      strcpy( s_loader_error, "RTUI_RENDER_COMMANDS expects a binary string" );
      hb_retc_null();
      return;
   }

   if( ! hrui_load() || s_render_commands == NULL )
   {
      if( s_library != NULL && s_render_commands == NULL )
      {
         strcpy( s_loader_error,
                 "This Ratatui DLL does not provide hrui_render_commands" );
      }
      hb_retc_null();
      return;
   }

   status = s_render_commands( ( const uint8_t * ) commands,
                               ( size_t ) commands_length,
                               ansi ? 1 : 0, NULL, 0, &required );
   if( status != HRUI_BUFFER_TOO_SMALL || required == 0 )
   {
      hb_retc_null();
      return;
   }

   output = ( char * ) hb_xgrab( ( HB_SIZE ) required + 1 );
   status = s_render_commands( ( const uint8_t * ) commands,
                               ( size_t ) commands_length,
                               ansi ? 1 : 0, ( uint8_t * ) output,
                               required, &written );
   if( status != HRUI_OK )
   {
      hb_xfree( output );
      hb_retc_null();
      return;
   }

   output[ written ] = '\0';
   s_loader_error[ 0 ] = '\0';
   hb_retclen_buffer( output, ( HB_SIZE ) written );
}

HB_FUNC( RTUI_SHOWCASE )
{
   uint32_t tick;
   size_t selected;
   int width;
   int height;
   HB_BOOL ansi;
   size_t required = 0;
   size_t written = 0;
   int32_t status;
   char * output;

   if( ! hrui_load() || s_showcase == NULL )
   {
      if( s_library != NULL && s_showcase == NULL )
      {
         strcpy( s_loader_error,
                 "This Ratatui DLL does not provide hrui_render_showcase" );
      }
      hb_retc_null();
      return;
   }

   tick = ( uint32_t ) hb_parnidef( 1, 0 );
   selected = ( size_t ) hb_parnidef( 2, 0 );
   width = hb_parnidef( 3, 120 );
   height = hb_parnidef( 4, 38 );
   ansi = hb_parldef( 5, HB_TRUE );

   status = s_showcase( tick, selected, ( uint16_t ) width,
                        ( uint16_t ) height, ansi ? 1 : 0,
                        NULL, 0, &required );
   if( status != HRUI_BUFFER_TOO_SMALL || required == 0 )
   {
      hb_retc_null();
      return;
   }

   output = ( char * ) hb_xgrab( ( HB_SIZE ) required + 1 );
   status = s_showcase( tick, selected, ( uint16_t ) width,
                        ( uint16_t ) height, ansi ? 1 : 0,
                        ( uint8_t * ) output, required, &written );
   if( status != HRUI_OK )
   {
      hb_xfree( output );
      hb_retc_null();
      return;
   }

   output[ written ] = '\0';
   s_loader_error[ 0 ] = '\0';
   hb_retclen_buffer( output, ( HB_SIZE ) written );
}

HB_FUNC( RTUI_SHOWCASE_EX )
{
   uint32_t tick;
   size_t selected;
   size_t menu;
   size_t menu_item;
   uint32_t checked_mask;
   HB_BOOL menu_open;
   int width;
   int height;
   HB_BOOL ansi;
   size_t required = 0;
   size_t written = 0;
   int32_t status;
   char * output;

   if( ! hrui_load() || s_showcase_v2 == NULL )
   {
      if( s_library != NULL && s_showcase_v2 == NULL )
      {
         strcpy( s_loader_error,
                 "This Ratatui DLL does not provide hrui_render_showcase_v2" );
      }
      hb_retc_null();
      return;
   }

   tick = ( uint32_t ) hb_parnidef( 1, 0 );
   selected = ( size_t ) hb_parnidef( 2, 0 );
   menu = ( size_t ) hb_parnidef( 3, 0 );
   menu_item = ( size_t ) hb_parnidef( 4, 0 );
   checked_mask = ( uint32_t ) hb_parnidef( 5, 63 );
   menu_open = hb_parldef( 6, HB_FALSE );
   width = hb_parnidef( 7, 120 );
   height = hb_parnidef( 8, 38 );
   ansi = hb_parldef( 9, HB_TRUE );

   status = s_showcase_v2( tick, selected, menu, menu_item,
                           checked_mask, menu_open ? 1 : 0,
                           ( uint16_t ) width, ( uint16_t ) height,
                           ansi ? 1 : 0, NULL, 0, &required );
   if( status != HRUI_BUFFER_TOO_SMALL || required == 0 )
   {
      hb_retc_null();
      return;
   }

   output = ( char * ) hb_xgrab( ( HB_SIZE ) required + 1 );
   status = s_showcase_v2( tick, selected, menu, menu_item,
                           checked_mask, menu_open ? 1 : 0,
                           ( uint16_t ) width, ( uint16_t ) height,
                           ansi ? 1 : 0, ( uint8_t * ) output,
                           required, &written );
   if( status != HRUI_OK )
   {
      hb_xfree( output );
      hb_retc_null();
      return;
   }

   output[ written ] = '\0';
   s_loader_error[ 0 ] = '\0';
   hb_retclen_buffer( output, ( HB_SIZE ) written );
}

HB_FUNC( RTUI_SHOWCASE_UI )
{
   uint32_t tick;
   size_t tree_selected;
   size_t table_selected;
   size_t focus;
   size_t menu;
   size_t menu_item;
   uint32_t checked_mask;
   HB_BOOL menu_open;
   int width;
   int height;
   HB_BOOL ansi;
   size_t required = 0;
   size_t written = 0;
   int32_t status;
   char * output;

   if( ! hrui_load() || s_showcase_v3 == NULL )
   {
      if( s_library != NULL && s_showcase_v3 == NULL )
      {
         strcpy( s_loader_error,
                 "This Ratatui DLL does not provide hrui_render_showcase_v3" );
      }
      hb_retc_null();
      return;
   }

   tick = ( uint32_t ) hb_parnidef( 1, 0 );
   tree_selected = ( size_t ) hb_parnidef( 2, 0 );
   table_selected = ( size_t ) hb_parnidef( 3, 0 );
   focus = ( size_t ) hb_parnidef( 4, 0 );
   menu = ( size_t ) hb_parnidef( 5, 0 );
   menu_item = ( size_t ) hb_parnidef( 6, 0 );
   checked_mask = ( uint32_t ) hb_parnidef( 7, 63 );
   menu_open = hb_parldef( 8, HB_FALSE );
   width = hb_parnidef( 9, 120 );
   height = hb_parnidef( 10, 38 );
   ansi = hb_parldef( 11, HB_TRUE );

   status = s_showcase_v3( tick, tree_selected, table_selected, focus,
                           menu, menu_item, checked_mask,
                           menu_open ? 1 : 0, ( uint16_t ) width,
                           ( uint16_t ) height, ansi ? 1 : 0,
                           NULL, 0, &required );
   if( status != HRUI_BUFFER_TOO_SMALL || required == 0 )
   {
      hb_retc_null();
      return;
   }

   output = ( char * ) hb_xgrab( ( HB_SIZE ) required + 1 );
   status = s_showcase_v3( tick, tree_selected, table_selected, focus,
                           menu, menu_item, checked_mask,
                           menu_open ? 1 : 0, ( uint16_t ) width,
                           ( uint16_t ) height, ansi ? 1 : 0,
                           ( uint8_t * ) output, required, &written );
   if( status != HRUI_OK )
   {
      hb_xfree( output );
      hb_retc_null();
      return;
   }

   output[ written ] = '\0';
   s_loader_error[ 0 ] = '\0';
   hb_retclen_buffer( output, ( HB_SIZE ) written );
}

HB_FUNC( RTUI_SHOWCASE_TREE )
{
   uint32_t tick;
   size_t tree_selected;
   size_t table_selected;
   size_t focus;
   size_t menu;
   size_t menu_item;
   uint32_t checked_mask;
   uint32_t expanded_mask;
   HB_BOOL menu_open;
   int width;
   int height;
   HB_BOOL ansi;
   size_t required = 0;
   size_t written = 0;
   int32_t status;
   char * output;

   if( ! hrui_load() || s_showcase_v4 == NULL )
   {
      if( s_library != NULL && s_showcase_v4 == NULL )
      {
         strcpy( s_loader_error,
                 "This Ratatui DLL does not provide hrui_render_showcase_v4" );
      }
      hb_retc_null();
      return;
   }

   tick = ( uint32_t ) hb_parnidef( 1, 0 );
   tree_selected = ( size_t ) hb_parnidef( 2, 0 );
   table_selected = ( size_t ) hb_parnidef( 3, 0 );
   focus = ( size_t ) hb_parnidef( 4, 0 );
   menu = ( size_t ) hb_parnidef( 5, 0 );
   menu_item = ( size_t ) hb_parnidef( 6, 0 );
   checked_mask = ( uint32_t ) hb_parnidef( 7, 63 );
   expanded_mask = ( uint32_t ) hb_parnidef( 8, 3 );
   menu_open = hb_parldef( 9, HB_FALSE );
   width = hb_parnidef( 10, 120 );
   height = hb_parnidef( 11, 38 );
   ansi = hb_parldef( 12, HB_TRUE );

   status = s_showcase_v4( tick, tree_selected, table_selected, focus,
                           menu, menu_item, checked_mask, expanded_mask,
                           menu_open ? 1 : 0, ( uint16_t ) width,
                           ( uint16_t ) height, ansi ? 1 : 0,
                           NULL, 0, &required );
   if( status != HRUI_BUFFER_TOO_SMALL || required == 0 )
   {
      hb_retc_null();
      return;
   }

   output = ( char * ) hb_xgrab( ( HB_SIZE ) required + 1 );
   status = s_showcase_v4( tick, tree_selected, table_selected, focus,
                           menu, menu_item, checked_mask, expanded_mask,
                           menu_open ? 1 : 0, ( uint16_t ) width,
                           ( uint16_t ) height, ansi ? 1 : 0,
                           ( uint8_t * ) output, required, &written );
   if( status != HRUI_OK )
   {
      hb_xfree( output );
      hb_retc_null();
      return;
   }

   output[ written ] = '\0';
   s_loader_error[ 0 ] = '\0';
   hb_retclen_buffer( output, ( HB_SIZE ) written );
}

HB_FUNC( RTUI_PRESENT )
{
   const char * utf8 = hb_parc( 1 );
   HB_SIZE utf8_length = hb_parclen( 1 );
   HRUI_HANDLE output = GetStdHandle( HRUI_STD_OUTPUT_HANDLE );
   HRUI_DWORD mode;
   int wide_length;
   uint16_t * wide;
   HRUI_DWORD offset;
   HRUI_DWORD written;
   HRUI_COORD home;
   HB_SIZE index;
   HB_BOOL ansi = hb_parldef( 2, HB_FALSE );

   if( utf8 == NULL )
   {
      strcpy( s_loader_error, "RTUI_PRESENT expects a UTF-8 string" );
      hb_retl( HB_FALSE );
      return;
   }

   /* Native presentation never accepts terminal control sequences. */
   for( index = 0; ! ansi && index < utf8_length; ++index )
   {
      if( ( unsigned char ) utf8[ index ] == 27 )
      {
         strcpy( s_loader_error,
                 "RTUI_PRESENT rejected an ANSI escape sequence" );
         hb_retl( HB_FALSE );
         return;
      }
   }

   if( output == HRUI_INVALID_HANDLE_VALUE )
   {
      hrui_set_windows_error( "GetStdHandle failed" );
      hb_retl( HB_FALSE );
      return;
   }

   /* Redirected stdout is already a byte stream; keep valid UTF-8 there. */
   if( ! GetConsoleMode( output, &mode ) )
   {
      if( fwrite( utf8, 1, ( size_t ) utf8_length, stdout ) !=
          ( size_t ) utf8_length )
      {
         strcpy( s_loader_error, "Could not write redirected UTF-8 output" );
         hb_retl( HB_FALSE );
         return;
      }
      fflush( stdout );
      s_loader_error[ 0 ] = '\0';
      hb_retl( HB_TRUE );
      return;
   }

   if( ansi &&
       ! SetConsoleMode( output,
                         mode | HRUI_ENABLE_VIRTUAL_TERMINAL_PROCESSING ) )
   {
      hrui_set_windows_error( "Could not enable Windows VT processing" );
      hb_retl( HB_FALSE );
      return;
   }

   wide_length = MultiByteToWideChar( HRUI_CP_UTF8,
                                      HRUI_MB_ERR_INVALID_CHARS,
                                      utf8, ( int ) utf8_length,
                                      NULL, 0 );
   if( wide_length <= 0 )
   {
      hrui_set_windows_error( "Invalid UTF-8 passed to RTUI_PRESENT" );
      hb_retl( HB_FALSE );
      return;
   }

   wide = ( uint16_t * ) hb_xgrab( ( HB_SIZE ) wide_length * sizeof( uint16_t ) );
   if( MultiByteToWideChar( HRUI_CP_UTF8, HRUI_MB_ERR_INVALID_CHARS,
                            utf8, ( int ) utf8_length,
                            wide, wide_length ) != wide_length )
   {
      hb_xfree( wide );
      hrui_set_windows_error( "UTF-8 to UTF-16 conversion failed" );
      hb_retl( HB_FALSE );
      return;
   }

   home.x = 0;
   home.y = 0;
   if( ! SetConsoleCursorPosition( output, home ) )
   {
      hb_xfree( wide );
      hrui_set_windows_error( "Could not position the console cursor" );
      hb_retl( HB_FALSE );
      return;
   }

   offset = 0;
   while( offset < ( HRUI_DWORD ) wide_length )
   {
      written = 0;
      if( ! WriteConsoleW( output, wide + offset,
                           ( HRUI_DWORD ) wide_length - offset,
                           &written, NULL ) || written == 0 )
      {
         hb_xfree( wide );
         hrui_set_windows_error( "WriteConsoleW failed" );
         hb_retl( HB_FALSE );
         return;
      }
      offset += written;
   }

   hb_xfree( wide );
   s_loader_error[ 0 ] = '\0';
   hb_retl( HB_TRUE );
}

HB_FUNC( RTUI_ENABLE_VT )
{
   HRUI_HANDLE output = GetStdHandle( HRUI_STD_OUTPUT_HANDLE );
   HRUI_DWORD mode;

   if( output == HRUI_INVALID_HANDLE_VALUE || ! GetConsoleMode( output, &mode ) )
   {
      hb_retl( HB_FALSE );
      return;
   }
   hb_retl( SetConsoleMode( output,
                           mode | HRUI_ENABLE_VIRTUAL_TERMINAL_PROCESSING ) != 0 );
}
