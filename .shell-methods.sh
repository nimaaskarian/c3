RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

if [ "$#" -ne 1 ]; then
  echo Usage: $0 tag_name
  exit 1
fi

echo_exit() {
 echo -e $RED"Error$NC: $*"
 exit 1 
}

echo_warning() {
 echo -e $YELLOW"Warning$NC: $*"
}
