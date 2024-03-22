RED='\033[0;31m'
NC='\033[0m' # No Color

if [ "$#" -ne 1 ]; then
  echo Usage: $0 tag_name
  exit 1
fi

echo_exit() {
 echo -e $RED"Error$NC: $*"
 exit 1 
}

