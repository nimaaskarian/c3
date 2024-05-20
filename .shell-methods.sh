RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo_exit() {
 echo -e $RED"Error$NC: $*"
 exit 1 
}

echo_warning() {
 echo -e $YELLOW"Warning$NC: $*"
}
