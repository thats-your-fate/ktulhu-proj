#!/usr/bin/perl
use strict;
use warnings;
use open qw(:std :utf8);
binmode(STDOUT, ":utf8");
use WWW::Curl::Easy;
use URI::Escape;
use JSON;

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

# --- CORE SCRAPE FUNCTION ---
sub scrape_query {
    my ($query) = @_;
    my $encoded_query = uri_escape($query);
    my $url = "https://duckduckgo.com/html/?q=$encoded_query";

    my $main_body = fetch_url($url);
    return { error => "Failed to fetch search results" } unless $main_body;

    my @results;
    while ($main_body =~ m|<a[^>]+class="result__a"[^>]+href="([^"]+)"[^>]*>(.*?)</a>|g) {
        my ($link, $text) = ($1, $2);
        $text =~ s/<[^>]+>//g;
        $link =~ s|^//|https://|;
        if ($link =~ /uddg=([^&]+)/) {
            $link = uri_unescape($1);
        }
        push @results, { link => $link, text => $text };
        last if @results >= 5;
    }

    my @items;
    for my $r (@results) {
        my %entry = ( title => $r->{text}, url => $r->{link} );
        my $page = fetch_url($r->{link});
        if ($page) {
            my ($h1) = $page =~ m|<h1[^>]*>(.*?)</h1>|is;
            my ($h2) = $page =~ m|<h2[^>]*>(.*?)</h2>|is;
            my $heading = $h1 || $h2 || '';
            $heading =~ s/<[^>]+>//g;
            $heading =~ s/^\s+|\s+$//g;
            $entry{headline} = $heading || '(no h1/h2 found)';

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
            $entry{paragraphs} = \@paras;
        } else {
            $entry{error} = "Failed to fetch page";
        }
        push @items, \%entry;
    }

    return { query => $query, results => \@items };
}

# --- MODE SELECTION ---
if (-t STDIN) {
    # Interactive or direct mode
    my $query = shift || 'perl web scraping';
    my $data = scrape_query($query);
    print "Top results for \"$query\":\n\n";
    my $i = 1;
    for my $r (@{$data->{results}}) {
        print "$i. $r->{title}\n   $r->{url}\n";
        print "   ➤ Headline: $r->{headline}\n";
        if ($r->{paragraphs} && @{$r->{paragraphs}}) {
            print "   ➤ Selected paragraphs:\n";
            my $c = 1;
            for my $p (@{$r->{paragraphs}}) {
                print "      [$c] $p\n\n";
                $c++;
            }
        } else {
            print "   [No suitable paragraphs]\n\n";
        }
        $i++;
    }
} else {
    # --- STDIN JSON PIPE MODE ---
    my $json = JSON->new->utf8->canonical;
    while (my $line = <STDIN>) {
        chomp $line;
        next unless $line =~ /\S/;
        my $result = scrape_query($line);
        print $json->encode($result) . "\n";
        STDOUT->flush;
    }
}
