#!/usr/bin/perl
use strict;
use warnings;
use WWW::Curl::Easy;
use URI::Escape;

# --- CONFIG ---
my $query = shift || 'perl web scraping';
my $encoded_query = uri_escape($query);
my $url = "https://duckduckgo.com/html/?q=$encoded_query";

# --- CURL SETUP ---
my $curl = WWW::Curl::Easy->new;
$curl->setopt(CURLOPT_FOLLOWLOCATION, 1);
$curl->setopt(CURLOPT_USERAGENT, 'Mozilla/5.0 (X11; Linux x86_64)');
$curl->setopt(CURLOPT_SSL_VERIFYPEER, 0);
$curl->setopt(CURLOPT_SSL_VERIFYHOST, 0);
$curl->setopt(CURLOPT_TIMEOUT, 10);

sub fetch_url {
    my ($target) = @_;
    my $body;
    $curl->setopt(CURLOPT_URL, $target);
    $curl->setopt(CURLOPT_WRITEDATA, \$body);
    my $ret = $curl->perform;
    return $ret == 0 ? $body : undef;
}

# --- FETCH DUCKDUCKGO ---
my $main_body = fetch_url($url);
if (!$main_body) {
    die "Failed to fetch search results.\n";
}

# --- PARSE RESULTS ---
my @results;
while ($main_body =~ m|<a[^>]+class="result__a"[^>]+href="([^"]+)"[^>]*>(.*?)</a>|g) {
    my ($link, $text) = ($1, $2);
    $text =~ s/<[^>]+>//g;

    # fix protocol
    $link =~ s|^//|https://|;

    # decode DuckDuckGo redirect links
    if ($link =~ /uddg=([^&]+)/) {
        my $decoded = uri_unescape($1);
        $link = $decoded;
    }

    push @results, { link => $link, text => $text };
    last if @results >= 5;
}

# --- OUTPUT ---
print "Top 5 results for \"$query\":\n\n";
my $i = 1;
for my $r (@results) {
    print "$i. $r->{text}\n   $r->{link}\n";

    my $page = fetch_url($r->{link});
    if ($page) {
        # --- Extract headline ---
        my ($h1) = $page =~ m|<h1[^>]*>(.*?)</h1>|is;
        my ($h2) = $page =~ m|<h2[^>]*>(.*?)</h2>|is;
        my $heading = $h1 || $h2 || '(no h1/h2 found)';
        $heading =~ s/<[^>]+>//g;
        $heading =~ s/^\s+|\s+$//g;
        print "   ➤ Headline: $heading\n";

        # --- Extract paragraphs (200–600 chars) ---
        my @paras;
        while ($page =~ m|<p[^>]*>(.*?)</p>|gis) {
            my $p = $1;
            $p =~ s/<[^>]+>//g;
            $p =~ s/\s+/ /g;
            $p =~ s/^\s+|\s+$//g;
            next if length($p) < 200 || length($p) > 600;
            push @paras, $p;
            last if @paras >= 7;
        }

        if (@paras) {
            print "   ➤ Selected paragraphs:\n";
            my $count = 1;
            for my $p (@paras) {
                print "      [$count] $p\n\n";
                $count++;
            }
        } else {
            print "   [No paragraphs between 200–600 chars found]\n";
        }

        print "\n";
    } else {
        print "   [Failed to fetch page]\n\n";
    }

    $i++;
}
